//! Immutable execution data in ProjectArchive. No runtime, authority or Engine is persisted.
use crate::{ApiError,ApiFieldError,ErrorCode,FrozenExecutionFile};
use crate::execution_projection::{ExecutionArchiveReader,ExecutionProjector,StoredExecutionSnapshot};
use crate::project_store::{ArchiveAddition,ProjectArchive,encode_compressed_document};
use sf_domain::{cloud_execution::*,*};
use serde::{Serialize,Deserialize};
use schemars::JsonSchema;
use ts_rs::TS;
use std::{collections::{BTreeMap,BTreeSet},sync::Arc};

#[derive(Debug,Clone,Serialize,Deserialize,JsonSchema,TS)]
#[serde(deny_unknown_fields)]
pub struct StoredExecutionFile {
    pub original_file:FileRef,
    pub stored_file:FileRef,
    pub byte_length:SafeUint,
}
#[derive(Debug,Clone,Serialize,Deserialize,JsonSchema,TS)]
#[serde(deny_unknown_fields)]
pub struct StoredExecutionArchive {
    #[schemars(with = "ExecutionFrozenInputs")]
    #[ts(as = "ExecutionFrozenInputs")]
    pub frozen_inputs:Arc<ExecutionFrozenInputs>,
    #[schemars(with = "Vec<ExecutionSnapshot>")]
    #[ts(as = "Vec<ExecutionSnapshot>")]
    pub snapshots:Vec<Arc<StoredExecutionSnapshot>>,
    // Unbuilt H01 preparation has frozen bytes but no actual r0/history yet.
    pub history:Option<ExecutionObservationHistoryDocument>,
    pub published_revisions:Vec<SafeUint>,
    pub files:Vec<StoredExecutionFile>,
}
pub(crate) struct ArchivedExecutionSource {
    pub record:Arc<StoredExecutionArchive>,
    pub reader:Option<ExecutionArchiveReader>,
    pub files:Arc<Vec<FrozenExecutionFile>>,
}
pub(crate) type ArchivedExecutions=BTreeMap<String,Arc<ArchivedExecutionSource>>;
pub(crate) fn key(identity:&ExecutionIdentity)->String {serde_json::to_string(identity).expect("typed execution identity")}
pub(crate) fn same<T:Serialize+?Sized>(a:&T,b:&T)->bool {serde_json::to_value(a).unwrap()==serde_json::to_value(b).unwrap()}
pub(crate) fn fail(message:&str)->ApiError {ApiError {code:ErrorCode::InvalidInput,message:message.into(),object_refs:vec![],
    field_errors:vec![ApiFieldError {pointer:"/execution_archive".into(),reason:message.into()}],expected_revision:None,actual_revision:None}}
fn document<T:for<'de>Deserialize<'de>>(bytes:&[u8])->Result<T,ApiError> {
    serde_json::from_slice::<Versioned<T>>(bytes).map(|v|v.data).map_err(|_|fail("Invalid versioned execution archive document"))
}
pub(crate) fn decode_archive_document(bytes:&[u8], path:&str)->Result<StoredExecutionArchive,ApiError> {
    if path.ends_with(".json.gz") {
        serde_json::from_reader::<_,Versioned<StoredExecutionArchive>>(std::io::BufReader::new(flate2::read::GzDecoder::new(bytes)))
            .map(|v|v.data).map_err(|_|fail("Invalid compressed versioned execution archive document"))
    } else { document(bytes) }
}
pub(crate) fn snapshot_prefix(old:&[Arc<StoredExecutionSnapshot>], new:&[Arc<StoredExecutionSnapshot>])->bool {
    let by_revision:BTreeMap<_,_>=new.iter().map(|s|(s.execution_revision.value(),s)).collect();
    old.iter().all(|s|by_revision.get(&s.execution_revision.value()).is_some_and(|n|s.same_snapshot(n)))
}
pub(crate) fn record_prefix(old:&[ExecutionObservationRecord],new:&[ExecutionObservationRecord])->bool {
    old.len()<=new.len() && old.iter().zip(new).all(|(a,b)|same(a,b))
}
pub(crate) fn same_history(old:Option<&ExecutionObservationHistoryDocument>,new:Option<&ExecutionObservationHistoryDocument>)->bool {
    match (old,new) {
        (None,None)=>true,
        (Some(a),Some(b))=>same(&(&a.document_kind,&a.contract_version,&a.identity,&a.observation_frontier),
            &(&b.document_kind,&b.contract_version,&b.identity,&b.observation_frontier))
            && a.records.len()==b.records.len() && record_prefix(&a.records,&b.records),
        _=>false,
    }
}
pub(crate) fn file_refs(record:&StoredExecutionArchive)->Vec<FileRef> {record.files.iter().map(|f|f.stored_file.clone()).collect()}

impl ArchivedExecutionSource {
    pub(crate) fn load(record:Arc<StoredExecutionArchive>,archive:&ProjectArchive)->Result<Self,ApiError> {
        let frozen=&record.frozen_inputs;
        if frozen.identity.project_id!=archive.project.id || frozen.project.id!=archive.project.id
            || frozen.project_input_ref.project_id!=archive.project.id
            || !same(&frozen.project.revision,&frozen.project_input_ref.project_revision)
            || frozen.scenario.id!=frozen.project_input_ref.scenario_id
            || !same(&frozen.scenario.revision,&frozen.project_input_ref.scenario_revision)
            || !same(&frozen.scenario.project_revision,&frozen.project.revision) {
            return Err(fail("Execution frozen project/scenario identity or revision conflicts"));
        }
        let mut manifest_by_path:BTreeMap<&str,Vec<&DocumentEntry>>=BTreeMap::new();
        for entry in &archive.manifest.documents {
            manifest_by_path.entry(entry.file_ref.path.as_str()).or_default().push(entry);
        }
        let mut original=BTreeSet::new();let mut stored=BTreeSet::new();let mut files=vec![];
        for binding in &record.files {
            if !original.insert(binding.original_file.path.as_str()) || !stored.insert(binding.stored_file.path.as_str())
                || !matches!(binding.stored_file.kind,DocumentKind::Asset) {
                return Err(fail("Duplicate or invalid execution frozen-file mapping"));
            }
            let entry=manifest_by_path.get(binding.stored_file.path.as_str())
                .and_then(|entries|entries.iter().copied().find(|e|same(&e.file_ref,&binding.stored_file)&&e.required))
                .ok_or_else(||fail("Execution frozen file has no required manifest reference"))?;
            let bytes=archive.files.get(entry.file_ref.path.as_str()).ok_or_else(||fail("Execution frozen file is missing"))?;
            if bytes.len() as u64!=binding.byte_length.value() {return Err(fail("Execution frozen file length differs from its original capture"));}
            files.push(FrozenExecutionFile {file:binding.original_file.clone(),bytes:bytes.clone()});
        }
        let files_by_original:BTreeMap<&str,&FrozenExecutionFile>=files.iter().map(|f|(f.file.path.as_str(),f)).collect();
        let get=|reference:&FileRef|->Result<&[u8],ApiError>{files_by_original.get(reference.path.as_str())
            .filter(|f|same(&f.file,reference)).map(|f|f.bytes.as_slice())
            .ok_or_else(||fail("Required original frozen reference is absent"))};
        let project:ProjectState=document(get(&frozen.project_input_ref.frozen_project_file)?)?;
        let scenario:ScenarioInput=document(get(&frozen.project_input_ref.frozen_scenario_file)?)?;
        if !same(&project,&frozen.project)||!same(&scenario,&frozen.scenario) {return Err(fail("Frozen project/scenario bytes differ from execution inputs"));}
        let mut modules=BTreeMap::new();
        for file in &files {
            match file.file.kind {
                DocumentKind::Module=>{
                    let module:ModulePackage=document(&file.bytes)?;
                    let mk=serde_json::to_string(&(module.definition.id.clone(),module.definition.version.clone())).unwrap();
                    if modules.insert(mk,module).is_some() {return Err(fail("Duplicate frozen module identity"));}
                },
                DocumentKind::Project|DocumentKind::Scenario|DocumentKind::Asset=>{},
                _=>return Err(fail("Unsupported kind in original execution frozen files")),
            }
        }
        if modules.len()!=frozen.modules.len() {return Err(fail("Incomplete frozen module byte closure"));}
        let module=|reference:&ModuleRef|modules.get(&serde_json::to_string(&(reference.id.clone(),reference.version.clone())).unwrap());
        let mut asset_bindings_by_module:BTreeMap<String,Vec<&ExecutionAssetBinding>>=BTreeMap::new();
        for binding in &frozen.asset_bindings {
            asset_bindings_by_module.entry(serde_json::to_string(&binding.module_ref).unwrap()).or_default().push(binding);
        }
        for package in &frozen.modules {
            let reference=ModuleRef {id:package.definition.id.clone(),version:package.definition.version.clone()};
            if module(&reference).is_none_or(|m|!same(m,package)) || package.dependencies.iter().any(|r|module(r).is_none()) {
                return Err(fail("Frozen exact module bytes or dependency closure conflict"));
            }
            for asset in &package.assets {
                let count=asset_bindings_by_module.get(&serde_json::to_string(&reference).unwrap())
                    .map_or(0,|bindings|bindings.iter().filter(|b|same(&b.module_ref,&reference)&&same(&b.asset,asset)).count());
                if count!=1 {
                    return Err(fail("Frozen module asset has no unique exact binding"));
                }
            }
        }
        if frozen.project.module_refs.iter().chain(frozen.project.devices.iter().flat_map(|d|&d.route_operation_refs)).any(|r|module(r).is_none())
            || frozen.execution_definitions.iter().any(|d|module(&ModuleRef{id:d.id.clone(),version:d.version.clone()}).is_none_or(|m|!same(&m.definition,d))) {
            return Err(fail("Execution definition closure differs from frozen modules"));
        }
        for asset in &frozen.asset_bindings {
            if !matches!(asset.archive_file.kind,DocumentKind::Asset) || module(&asset.module_ref).is_none_or(|m|!m.assets.iter().any(|a|same(a,&asset.asset))) {
                return Err(fail("Frozen asset module/version or reference is undeclared"));
            }
            get(&asset.archive_file)?;
        }
        let mut revisions=BTreeSet::new();
        for snapshot in &record.snapshots {
            if !same(&snapshot.identity,&frozen.identity)||!same(&snapshot.project_input_ref,&frozen.project_input_ref)
                || !revisions.insert(snapshot.execution_revision.value()) {
                return Err(fail("Snapshot identity/input/revision differs from its original execution"));
            }
        }
        let mut published=BTreeSet::new();
        if record.published_revisions.iter().any(|r|!revisions.contains(&r.value())||!published.insert(r.value())) {
            return Err(fail("Saved publication stamp is not a unique saved execution revision"));
        }
        let reader=if record.snapshots.is_empty() {
            if record.history.is_some()||!record.published_revisions.is_empty() {return Err(fail("Unbuilt preparation cannot claim snapshots/history/publication"));}
            None
        } else {
            let history=record.history.clone().ok_or_else(||fail("Saved execution has no complete history"))?;
            if !same(&history.identity,&frozen.identity) {return Err(fail("History belongs to another execution identity"));}
            Some(ExecutionProjector::from_archive(history,record.snapshots.clone()).map_err(|e|fail(&e.message))?)
        };
        Ok(Self {record,reader,files:Arc::new(files)})
    }
    pub(crate) fn is_prefix_of(&self,new:&Self)->bool {
        if !same(&self.record.frozen_inputs,&new.record.frozen_inputs)||self.files.len()!=new.files.len()
            || self.files.iter().any(|old|new.files.iter().find(|n|same(&n.file,&old.file)).is_none_or(|n|n.bytes!=old.bytes)) {return false;}
        if !snapshot_prefix(&self.record.snapshots,&new.record.snapshots) {return false;}
        if let Some(old)=&self.record.history {
            let Some(new)=&new.record.history else{return false};
            if !record_prefix(&old.records,&new.records) {return false;}
        }
        self.record.published_revisions.iter().all(|r|new.record.published_revisions.contains(r))
    }
}
pub(crate) fn load_sources(archive:&ProjectArchive,documents:&BTreeMap<String,Arc<StoredExecutionArchive>>)->Result<ArchivedExecutions,ApiError> {
    let mut result=ArchivedExecutions::new();
    for entry in &archive.manifest.documents {
        if !matches!(entry.file_ref.kind,DocumentKind::ExecutionArchive) {continue;}
        if !entry.required||!matches!(entry.role,DocumentRole::SupportingFile)||!matches!(entry.object_ref.kind,ObjectKind::Project)
            ||entry.object_ref.id.as_str()!=archive.project.id.as_str()||entry.revision.is_some() {return Err(fail("Execution archive manifest owner/role conflicts"));}
        let bytes=archive.files.get(entry.file_ref.path.as_str()).ok_or_else(||fail("Execution archive document is missing"))?;
        let _ = bytes; // Presence checked above; reuse the already parsed immutable document.
        let record=documents.get(entry.file_ref.path.as_str()).ok_or_else(||fail("Execution archive document was not parsed"))?.clone();
        let source=Arc::new(ArchivedExecutionSource::load(record,archive)?);
        let key=key(&source.record.frozen_inputs.identity);
        if let Some(old)=result.get(&key) {
            if old.is_prefix_of(&source) {result.insert(key,source);}
            else if !source.is_prefix_of(old) {return Err(fail("Saved records for one full execution identity are conflicting prefixes"));}
        } else {result.insert(key,source);}
    }
    Ok(result)
}

pub(crate) struct ExecutionArchiveCapture {
    pub directory:String,
    pub frozen:Arc<ExecutionFrozenInputs>,
    pub snapshots:Vec<Arc<StoredExecutionSnapshot>>,
    pub history:Option<ExecutionObservationHistoryDocument>,
    pub published:Vec<SafeUint>,
    pub files:Arc<Vec<FrozenExecutionFile>>,
    pub inherited:Option<Vec<StoredExecutionFile>>,
}
impl ExecutionArchiveCapture {
    /// Runs in the existing I/O worker, including all encoding and byte copies.
    pub(crate) fn additions(self)->Result<Vec<ArchiveAddition>,ApiError> {
        let owner=ObjectRef {kind:ObjectKind::Project,id:self.frozen.identity.project_id.as_str().to_owned().try_into().unwrap()};
        let mut additions=vec![];let mut bindings=vec![];
        for (i,file) in self.files.iter().enumerate() {
            if let Some(old)=self.inherited.as_ref().and_then(|xs|xs.iter().find(|x|same(&x.original_file,&file.file))) {
                bindings.push(old.clone());continue;
            }
            let stored=FileRef {path:format!("{}/frozen-{i:04}.bin",self.directory).try_into().unwrap(),kind:DocumentKind::Asset};
            let asset=self.frozen.asset_bindings.iter().find(|b|same(&b.archive_file,&file.file));
            let object=asset.map_or_else(||owner.clone(),|b|ObjectRef {kind:ObjectKind::Asset,id:b.asset.id.as_str().to_owned().try_into().unwrap()});
            additions.push(ArchiveAddition {entry:DocumentEntry {role:DocumentRole::SupportingFile,module_ref:None,object_ref:object,
                revision:None,file_ref:stored.clone(),required:true},bytes:file.bytes.clone()});
            bindings.push(StoredExecutionFile {original_file:file.file.clone(),stored_file:stored,byte_length:(file.bytes.len() as u64).try_into().map_err(|_|fail("Frozen file exceeds safe length"))?});
        }
        let record=StoredExecutionArchive {frozen_inputs:self.frozen,snapshots:self.snapshots,history:self.history,published_revisions:self.published,files:bindings};
        additions.push(ArchiveAddition {entry:DocumentEntry {role:DocumentRole::SupportingFile,module_ref:None,object_ref:owner,revision:None,
            file_ref:FileRef {path:format!("{}/source.json.gz",self.directory).try_into().unwrap(),kind:DocumentKind::ExecutionArchive},required:true},bytes:encode_compressed_document(&record)});
        Ok(additions)
    }
}
