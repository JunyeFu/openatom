// Generated from Rust DesktopCloseState; private main-window lifecycle DTO.
export type DesktopCloseState = { project_id: string | null, project_revision: number | null, unsaved: boolean, definition_revisions: Array<[string, number]>, active_tasks: Array<string>, execution_blockers: Array<{ session_id: string, execution_epoch_id: string, execution_revision: number, reason: string }>, };
