use lsp_server::ErrorCode;
use lsp_types::notification::DidCloseTextDocument;
use lsp_types::DidCloseTextDocumentParams;
use std::path::PathBuf;

// use crate::server::api::diagnostics::clear_diagnostics;
use crate::server::api::traits::{NotificationHandler, SyncNotificationHandler};
// use crate::server::api::LSPResult;
use crate::server::api::LSPResult;
use crate::server::api::ResultExt;
use crate::server::client::{Notifier, Requester};
use crate::server::Result;
use crate::session::Session;
use crate::DocumentKey;
// use crate::system::{url_to_any_system_path, AnySystemPath};

pub(crate) struct DidCloseTextDocumentHandler;

impl NotificationHandler for DidCloseTextDocumentHandler {
    type NotificationType = DidCloseTextDocument;
}

impl SyncNotificationHandler for DidCloseTextDocumentHandler {
    fn run(
        session: &mut Session,
        _notifier: Notifier,
        _requester: &mut Requester,
        params: DidCloseTextDocumentParams,
    ) -> Result<()> {
        tracing::info!("DidCloseTextDocumentHandler");

        match session.default_project_db() {
            None => {}
            Some(project) => {
                let document_key = DocumentKey::from_url(
                    &PathBuf::from(project.root_path()),
                    &params.text_document.uri,
                )
                .internal_error()?;
                session
                    .close_document(&document_key)
                    .with_failure_code(ErrorCode::InternalError)?;
            }
        }

        // if let AnySystemPath::SystemVirtual(virtual_path) = path {
        //     let db = session.default_project_db_mut();
        //     db.apply_changes(vec![ChangeEvent::DeletedVirtual(virtual_path)], None);
        // }

        // clear_diagnostics(key.url(), &notifier)?;

        Ok(())
    }
}
