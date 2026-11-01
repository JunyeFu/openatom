#[cfg(windows)]
mod windows {
    use rmcp::{
        ErrorData as McpError, RoleServer, ServerHandler, ServiceExt, model::*,
        service::RequestContext,
    };
    use serde_json::{Value, json};
    use sf_commands::{ApiRequest, CapabilityCatalog};
    use sf_mcp_bridge::{catalog, client::HostClient};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    #[derive(Clone)]
    struct Bridge {
        client: Arc<Mutex<HostClient>>,
    }
    impl Bridge {
        async fn exchange(
            &self,
            request: &Value,
            context: &RequestContext<RoleServer>,
        ) -> sf_mcp_bridge::decode::Decoded {
            tokio::select! {
                _ = context.ct.cancelled() => {
                    eprintln!("MCP request cancelled: closing transport; accepted Host work is not resubmitted or cancelled");
                    std::process::exit(75);
                }
                result = async { self.client.lock().await.call(request, || context.ct.is_cancelled()).await } => match result {
                    Ok(response) => response,
                    Err(error) => {
                        // Exit also closes a duplicated lease whose frame was cut short.
                        eprintln!("Host transport closed: {error}; no request was replayed");
                        std::process::exit(76);
                    }
                }
            }
        }
    }
    impl ServerHandler for Bridge {
        fn get_info(&self) -> ServerConfig {
            ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
                .with_instructions("Use only advertised shared Host capabilities and actual principal grants. Every tool takes the canonical ApiRequest including operation, project_id, scope and revision. Long actions return admission TaskHandle: read the original get_task for completion. Managed results include the complete original Response as application/json resource text. Transport cancellation never means business cancellation. Never retry ambiguous H01/H03/H10/H11; retain the original request and use H09 request recovery where supported, with fresh desktop authorization after reconnect. Prepared is not running; run/control require explicit user authorization and actual latest Host-owner access at the original expectedR. Keep exact execution session/epoch/R/F/T and full fixed-filter pages; goal satisfied is not terminal with unsettled obligations. Exiting observation never stops production. H10/H11 require their actual separate directory-selection purposes. Device Gateway operations/credentials are private. Missing execution providers cannot be replaced by run_scenario.")
        }
        async fn list_tools(
            &self,
            request: Option<PaginatedRequestParams>,
            context: RequestContext<RoleServer>,
        ) -> Result<ListToolsResult, McpError> {
            if request.and_then(|r| r.cursor).is_some() {
                return Err(McpError::invalid_params(
                    "This Host catalog is not paginated",
                    None,
                ));
            }
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
            let id = format!(
                "bridge-capabilities-{}",
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            );
            let response = self.exchange(&json!({"api_version":"0.1","request_id":id,"project_id":null,"scope":{"kind":"workspace"},"operation":"describe_capabilities","params":{}}), &context).await;
            if response.original["outcome"]["status"] != "ok" {
                return Err(McpError::internal_error(
                    "Host did not authorize capability discovery",
                    Some(response.original),
                ));
            }
            let capability: CapabilityCatalog =
                serde_json::from_value(response.original["outcome"]["data"].clone())
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Ok(ListToolsResult {
                tools: catalog::tools(&capability)
                    .map_err(|e| McpError::internal_error(e.to_string(), None))?,
                ..Default::default()
            })
        }
        async fn call_tool(
            &self,
            request: CallToolRequestParams,
            context: RequestContext<RoleServer>,
        ) -> Result<CallToolResponse, McpError> {
            let value = Value::Object(request.arguments.unwrap_or_default());
            let _: ApiRequest = serde_json::from_value(value.clone())
                .map_err(|e| McpError::invalid_params(e.to_string(), None))?;
            if request.name.as_ref()
                != format!(
                    "superfactory_{}",
                    value["operation"].as_str().unwrap_or_default()
                )
            {
                return Err(McpError::invalid_params(
                    "Tool name and canonical operation differ",
                    None,
                ));
            }
            let result = self
                .exchange(&value, &context)
                .await
                .mcp()
                .map_err(|e| McpError::internal_error(e.to_string(), None))?;
            Ok(result.into())
        }
    }
    pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
        let arguments: Vec<_> = std::env::args().skip(1).collect();
        if arguments == ["--version"] {
            println!("superfactory-mcp-bridge {}", env!("CARGO_PKG_VERSION"));
            return Ok(());
        }
        if arguments.len() != 4
            || arguments[0] != "--host-pipe"
            || arguments[2] != "--host-instance-id"
        {
            return Err("Usage: superfactory-mcp-bridge --host-pipe <endpoint> --host-instance-id <desktop-host-instance-id>".into());
        }
        let client = HostClient::connect(&arguments[1], &arguments[3]).await?;
        let service = Bridge {
            client: Arc::new(Mutex::new(client)),
        }
        .serve(rmcp::transport::stdio())
        .await?;
        service.waiting().await?;
        Ok(())
    }
}
#[cfg(windows)]
#[tokio::main]
async fn main() {
    if let Err(error) = windows::run().await {
        eprintln!("superfactory-mcp-bridge: {error}");
        std::process::exit(2);
    }
}
#[cfg(not(windows))]
fn main() {
    eprintln!("This native connector requires Windows named-pipe peer authentication");
    std::process::exit(2);
}
