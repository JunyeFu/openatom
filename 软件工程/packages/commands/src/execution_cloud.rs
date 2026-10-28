//! Fixed-origin, Native-private cloud gateway. All HTTP methods are called by
//! the worker outside the Host/Engine lock. No serializer exposes credentials.
use crate::{ApiError, CloudEffectAck, CloudWorkItem, ErrorCode};
use reqwest::{Method, Url, blocking::Client, header::HeaderMap};
use serde::de::DeserializeOwned;
use sf_domain::{cloud_execution::*, *};

type Result<T> = std::result::Result<T, ApiError>;
fn error(code: ErrorCode, message: &str) -> ApiError {
    ApiError {
        code,
        message: message.to_owned(),
        object_refs: vec![],
        field_errors: vec![],
        expected_revision: None,
        actual_revision: None,
    }
}
fn same(a: &impl serde::Serialize, b: &impl serde::Serialize) -> bool {
    serde_json::to_value(a).unwrap() == serde_json::to_value(b).unwrap()
}

pub struct ExecutionCloudConnection {
    pub(crate) connection_ref: ObjectId,
    pub(crate) project_id: ProjectId,
    pub(crate) cloud_project_id: CloudProjectId,
    endpoint: Url,
    token: String,
    client: Client,
}
pub struct CloudIngressPage {
    pub page: CloudEventsPage,
    pub visible_frontier: SafeUint,
}
impl ExecutionCloudConnection {
    pub fn https(
        connection_ref: ObjectId,
        project_id: ProjectId,
        cloud_project_id: CloudProjectId,
        endpoint: String,
        token: String,
    ) -> Result<Self> {
        Self::build(
            connection_ref,
            project_id,
            cloud_project_id,
            endpoint,
            token,
            false,
        )
    }
    /// Explicit component integration mode. It never permits public HTTP.
    pub fn local_loopback(
        connection_ref: ObjectId,
        project_id: ProjectId,
        cloud_project_id: CloudProjectId,
        endpoint: String,
        token: String,
    ) -> Result<Self> {
        Self::build(
            connection_ref,
            project_id,
            cloud_project_id,
            endpoint,
            token,
            true,
        )
    }
    fn build(
        connection_ref: ObjectId,
        project_id: ProjectId,
        cloud_project_id: CloudProjectId,
        endpoint: String,
        token: String,
        local: bool,
    ) -> Result<Self> {
        let endpoint = Url::parse(&endpoint)
            .map_err(|_| error(ErrorCode::InvalidInput, "Invalid private cloud origin"))?;
        let loopback = local
            && endpoint.scheme() == "http"
            && matches!(endpoint.host_str(), Some("127.0.0.1" | "::1" | "[::1]"));
        if (endpoint.scheme() != "https" && !loopback)
            || !endpoint.username().is_empty()
            || endpoint.password().is_some()
            || endpoint.query().is_some()
            || endpoint.fragment().is_some()
            || endpoint.path() != "/"
            || endpoint.host_str().is_none()
            || token.is_empty()
            || token.contains(['\r', '\n'])
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "A fixed private HTTPS origin and gateway credential are required",
            ));
        }
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|_| {
                error(
                    ErrorCode::IoFailure,
                    "Cloud TLS client initialization failed",
                )
            })?;
        Ok(Self {
            connection_ref,
            project_id,
            cloud_project_id,
            endpoint,
            token,
            client,
        })
    }
    fn url(&self, path: &[&str]) -> Url {
        let mut url = self.endpoint.clone();
        url.path_segments_mut().unwrap().clear().extend(path);
        url
    }
    fn identity(&self, identity: &ExecutionIdentity) -> Result<()> {
        if identity.project_id != self.project_id {
            return Err(error(
                ErrorCode::PermissionDenied,
                "Execution is outside the Native cloud configuration",
            ));
        }
        Ok(())
    }
    fn request(
        &self,
        method: Method,
        path: &[&str],
        query: &[(&str, String)],
        body: Option<&impl serde::Serialize>,
    ) -> Result<(CloudResponseVariant0Result, HeaderMap)> {
        let mut request = self
            .client
            .request(method, self.url(path))
            .bearer_auth(&self.token)
            .query(query);
        let large_execution_document = (path.get(1) == Some(&"projects") && path.get(3) == Some(&"sessions"))
            || (path.get(1) == Some(&"sessions") && path.get(3) == Some(&"snapshots"));
        if let Some(body) = body {
            if large_execution_document {
                use std::io::Write;
                let mut gzip = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
                serde_json::to_writer(&mut gzip, body).map_err(|_| error(ErrorCode::IoFailure, "Cannot encode frozen execution upload"))?;
                gzip.flush().map_err(|_| error(ErrorCode::IoFailure, "Cannot compress frozen execution upload"))?;
                let bytes = gzip.finish().map_err(|_| error(ErrorCode::IoFailure, "Cannot finish frozen execution upload"))?;
                request = request.header("Content-Encoding", "gzip").header("Content-Type", "application/json").body(bytes);
            } else {
                request = request.json(body);
            }
        }
        // Full-factory inputs and coalesced observation prefixes both need upload
        // and transactional storage time; retain the same request on an unknown result.
        if large_execution_document {
            request = request.timeout(std::time::Duration::from_secs(120));
        }
        let response = request.send().map_err(|cause| {
            error(
                ErrorCode::IoFailure,
                if cause.is_timeout() {
                    "Cloud request timed out; retain the original work identity"
                } else {
                    "Cloud transport result is unknown; retain the original work identity"
                },
            )
        })?;
        let status = response.status();
        let headers = response.headers().clone();
        let response: CloudResponse = response.json().map_err(|_| {
            error(
                ErrorCode::IoFailure,
                "Cloud returned an invalid execution response",
            )
        })?;
        if !status.is_success() {
            return Err(error(
                match status.as_u16() {
                    401 | 403 => ErrorCode::PermissionDenied,
                    409 => ErrorCode::RevisionConflict,
                    422 => ErrorCode::InvalidInput,
                    _ => ErrorCode::IoFailure,
                },
                &format!("Cloud HTTP {} did not confirm the original operation; its pending obligations are retained", status.as_u16()),
            ));
        }
        let CloudResponse::Variant0(success) = response else {
            return Err(error(
                ErrorCode::IoFailure,
                "Cloud returned an error with successful HTTP status",
            ));
        };
        Ok((success.result, headers))
    }
    fn data<T: DeserializeOwned>(result: CloudResponseVariant0Result, kind: &str) -> Result<T> {
        let mut v = serde_json::to_value(result).unwrap();
        if v["kind"] != kind {
            return Err(error(
                ErrorCode::IoFailure,
                "Cloud result kind differs from the original gateway work",
            ));
        }
        serde_json::from_value(v["data"].take()).map_err(|_| {
            error(
                ErrorCode::IoFailure,
                "Cloud gateway result has an incompatible shape",
            )
        })
    }
    pub fn create_session(
        &self,
        input: &CloudSessionCreate,
    ) -> Result<CloudResponseVariant0ResultSessionStoredData> {
        self.identity(&input.identity)?;
        if !same(&input.identity, &input.frozen_inputs.identity)
            || !same(
                &input.project_input_ref,
                &input.frozen_inputs.project_input_ref,
            )
            || !same(
                &input.candidate_bindings,
                &input.frozen_inputs.candidate_bindings,
            )
        {
            return Err(error(
                ErrorCode::InvalidInput,
                "Prepared cloud input does not name the same frozen execution",
            ));
        }
        let (result, _) = self.request(
            Method::PUT,
            &[
                "v2",
                "projects",
                self.cloud_project_id.as_str(),
                "sessions",
                input.identity.session_id.as_str(),
            ],
            &[],
            Some(input),
        )?;
        let ack: CloudResponseVariant0ResultSessionStoredData =
            Self::data(result, "session_stored")?;
        if ack.session_id != input.identity.session_id
            || ack.execution_epoch_id != input.identity.execution_epoch_id
        {
            return Err(error(
                ErrorCode::IoFailure,
                "Cloud acknowledged another session or epoch",
            ));
        }
        Ok(ack)
    }
    pub fn active_execution(&self) -> Result<ActiveExecutionSlot> {
        let (result, _) = self.request(Method::GET, &["v2", "projects", self.cloud_project_id.as_str(), "active-execution"], &[], None::<&()>)?;
        let slot: ActiveExecutionSlot = Self::data(result, "active_execution")?;
        if slot.cloud_project_id != self.cloud_project_id {
            return Err(error(ErrorCode::IoFailure, "Cloud returned another active project slot"));
        }
        Ok(slot)
    }
    pub fn publish_work(&self, work: &CloudWorkItem) -> Result<CloudEffectAck> {
        match work {
            CloudWorkItem::PublishCommand { work_id, command } => {
                self.identity(&command.identity)?;
                let (result, _) = self.request(
                    Method::PUT,
                    &[
                        "v2",
                        "sessions",
                        command.identity.session_id.as_str(),
                        "commands",
                        command.attempt.command_id.as_str(),
                    ],
                    &[],
                    Some(command),
                )?;
                let ack: CloudResponseVariant0ResultCommandStoredData =
                    Self::data(result, "command_stored")?;
                if ack.command_id != command.attempt.command_id {
                    return Err(error(
                        ErrorCode::IoFailure,
                        "Cloud acknowledged another command",
                    ));
                }
                Ok(CloudEffectAck::CommandStored {
                    work_id: work_id.clone(),
                    command_id: ack.command_id,
                    storage_status: ack.storage_status,
                })
            }
            CloudWorkItem::PublishSnapshot {
                work_id,
                publication,
            } => {
                self.identity(&publication.snapshot.identity)?;
                let identity = &publication.snapshot.identity;
                let revision = publication.snapshot.execution_revision.value().to_string();
                let (result, _) = self.request(
                    Method::PUT,
                    &[
                        "v2",
                        "sessions",
                        identity.session_id.as_str(),
                        "snapshots",
                        &revision,
                    ],
                    &[],
                    Some(publication),
                )?;
                let ack: PublishAck = Self::data(result, "snapshot_published")?;
                if ack.session_id != identity.session_id
                    || ack.execution_epoch_id != identity.execution_epoch_id
                    || ack.execution_revision.value()
                        != publication.snapshot.execution_revision.value()
                    || ack.observation_frontier.value()
                        != publication.snapshot.observation_frontier.value()
                {
                    return Err(error(
                        ErrorCode::IoFailure,
                        "Cloud snapshot acknowledgement differs from the original revision/frontier",
                    ));
                }
                Ok(CloudEffectAck::SnapshotPublished {
                    work_id: work_id.clone(),
                    ack,
                })
            }
            CloudWorkItem::WithdrawCommand {
                work_id,
                identity,
                command_id,
            } => {
                self.identity(identity)?;
                let input = CloudWithdrawRequest {
                    contract_version: CloudWithdrawRequestContractVersion::SfExecution20Draft11,
                    identity: identity.clone(),
                    command_id: command_id.clone(),
                };
                let (result, _) = self.request(
                    Method::POST,
                    &["v2", "commands", command_id.as_str(), "withdraw"],
                    &[],
                    Some(&input),
                )?;
                let ack: CloudResponseVariant0ResultWithdrawalData =
                    Self::data(result, "withdrawal")?;
                if ack.command_id != *command_id {
                    return Err(error(
                        ErrorCode::IoFailure,
                        "Cloud withdrawal acknowledges another command",
                    ));
                }
                Ok(CloudEffectAck::WithdrawalResult {
                    work_id: work_id.clone(),
                    command_id: ack.command_id,
                    state: ack.state,
                })
            }
        }
    }
    pub fn events(
        &self,
        identity: &ExecutionIdentity,
        after: SafeUint,
        limit: PageLimit,
        visible_frontier: Option<SafeUint>,
    ) -> Result<CloudIngressPage> {
        self.identity(identity)?;
        let mut query = vec![
            (
                "execution_epoch_id",
                identity.execution_epoch_id.as_str().to_owned(),
            ),
            ("after_ingress_id", after.value().to_string()),
            ("limit", limit.value().to_string()),
        ];
        if let Some(f) = &visible_frontier {
            query.push(("visible_frontier", f.value().to_string()));
        }
        let (result, headers) = self.request(
            Method::GET,
            &["v2", "sessions", identity.session_id.as_str(), "events"],
            &query,
            None::<&()>,
        )?;
        let page: CloudEventsPage = Self::data(result, "events_page")?;
        let frontier = headers
            .get("X-SF-Visible-Frontier")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.parse::<u64>().ok())
            .and_then(|v| SafeUint::try_from(v).ok())
            .ok_or_else(|| {
                error(
                    ErrorCode::IoFailure,
                    "Cloud events response omitted its actual fixed frontier",
                )
            })?;
        if !same(&page.identity, identity)
            || visible_frontier
                .as_ref()
                .is_some_and(|f| f.value() != frontier.value())
            || after.value() > frontier.value()
            || page.rows.len() > limit.value() as usize
        {
            return Err(error(
                ErrorCode::IoFailure,
                "Cloud event page changed its fixed selection",
            ));
        }
        let mut previous = after.value();
        for row in &page.rows {
            let value = serde_json::to_value(row).unwrap();
            let ingress = value["ingress_id"].as_u64().unwrap();
            if ingress <= previous || ingress > frontier.value() {
                return Err(error(
                    ErrorCode::IoFailure,
                    "Cloud event ingress order is inconsistent",
                ));
            }
            previous = ingress;
        }
        if page
            .next_ingress_id
            .as_ref()
            .is_some_and(|n| page.rows.is_empty() || n.value() != previous)
        {
            return Err(error(
                ErrorCode::IoFailure,
                "Cloud events cursor does not follow this page",
            ));
        }
        Ok(CloudIngressPage {
            page,
            visible_frontier: frontier,
        })
    }
    pub fn retire_boot(
        &self,
        identity: &ExecutionIdentity,
        device: &ObjectId,
        boot: &DeviceBootId,
        published_revision: SafeUint,
    ) -> Result<DeviceBoot> {
        self.identity(identity)?;
        let input = DeviceBootRetireRequest {
            contract_version: DeviceBootRetireRequestContractVersion::SfExecution20Draft11,
            session_id: identity.session_id.clone(),
            execution_epoch_id: identity.execution_epoch_id.clone(),
            expected_published_revision: published_revision,
        };
        let (result, _) = self.request(
            Method::POST,
            &[
                "v2",
                "devices",
                device.as_str(),
                "boots",
                boot.as_str(),
                "retire",
            ],
            &[],
            Some(&input),
        )?;
        let ack: DeviceBoot = Self::data(result, "device_boot")?;
        if ack.device_id != *device
            || ack.device_boot_id != *boot
            || ack.session_id != identity.session_id
            || ack.execution_epoch_id != identity.execution_epoch_id
            || !matches!(ack.state, DeviceBootState::Retired)
        {
            return Err(error(
                ErrorCode::IoFailure,
                "Cloud retirement proof differs from the original boot",
            ));
        }
        Ok(ack)
    }
    pub fn release_slot(
        &self,
        input: &CloudReleaseRequest,
    ) -> Result<CloudResponseVariant0ResultSlotReleasedData> {
        self.identity(&input.identity)?;
        let (result, _) = self.request(
            Method::POST,
            &[
                "v2",
                "sessions",
                input.identity.session_id.as_str(),
                "release-slot",
            ],
            &[],
            Some(input),
        )?;
        let ack: CloudResponseVariant0ResultSlotReleasedData = Self::data(result, "slot_released")?;
        if ack.session_id != input.identity.session_id
            || ack.execution_epoch_id != input.identity.execution_epoch_id
        {
            return Err(error(
                ErrorCode::IoFailure,
                "Cloud released another session slot",
            ));
        }
        Ok(ack)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn connection(url: &str, local: bool) -> Result<ExecutionCloudConnection> {
        ExecutionCloudConnection::build(
            "native-private".to_owned().try_into().unwrap(),
            "P".to_owned().try_into().unwrap(),
            "C".to_owned().try_into().unwrap(),
            url.into(),
            "private-test-gateway".into(),
            local,
        )
    }
    #[test]
    fn gateway_origin_and_encoded_identity_stay_in_the_native_boundary() {
        for url in [
            "http://example.invalid/",
            "https://user:secret@example.invalid/",
            "https://example.invalid/base",
            "https://example.invalid/?token=x",
        ] {
            assert!(connection(url, false).is_err());
        }
        assert!(connection("http://localhost:23270/", true).is_err());
        let c = connection("http://127.0.0.1:23270/", true).unwrap();
        assert_eq!(
            c.url(&["v2", "commands", "one/two+设备"]).path(),
            "/v2/commands/one%2Ftwo+%E8%AE%BE%E5%A4%87"
        );
    }
    #[test]
    fn gateway_refuses_another_local_project_before_network_io() {
        let c = connection("https://example.invalid/", false).unwrap();
        let identity=serde_json::from_value(serde_json::json!({"session_id":"S","execution_epoch_id":"E","project_id":"foreign","host_context_session_id":"H"})).unwrap();
        let e = c
            .events(
                &identity,
                SafeUint::try_from(0_u64).unwrap(),
                PageLimit::try_from(200_u64).unwrap(),
                None,
            )
            .err()
            .unwrap();
        assert!(matches!(e.code, ErrorCode::PermissionDenied));
        assert!(!e.message.contains("private-test-gateway"));
    }
}
