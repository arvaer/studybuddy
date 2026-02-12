use uuid::Uuid;

use domain::repository_traits::NoteRepository;

use crate::dtos::note::{CreateNoteRequest, NoteResponse, UpdateNoteRequest};
use crate::errors::AppError;

pub struct NoteService<R: NoteRepository> {
    repo: R,
}

impl<R: NoteRepository> NoteService<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    pub async fn list(
        &self,
        user_id: Uuid,
        concept_id: Option<Uuid>,
        ru_id: Option<Uuid>,
    ) -> Result<Vec<NoteResponse>, AppError> {
        let notes = self.repo.list(user_id, concept_id, ru_id).await?;
        Ok(notes.into_iter().map(NoteResponse::from).collect())
    }

    pub async fn create(
        &self,
        user_id: Uuid,
        req: CreateNoteRequest,
    ) -> Result<NoteResponse, AppError> {
        let concept_id = Uuid::parse_str(&req.concept_id)
            .map_err(|_| AppError::Validation("invalid concept_id".into()))?;

        let ru_id = req
            .ru_id
            .as_deref()
            .map(Uuid::parse_str)
            .transpose()
            .map_err(|_| AppError::Validation("invalid ru_id".into()))?;

        let note = self
            .repo
            .create(user_id, concept_id, ru_id, &req.content, false, req.anchor_position)
            .await?;
        Ok(NoteResponse::from(note))
    }

    pub async fn update(
        &self,
        id: Uuid,
        user_id: Uuid,
        req: UpdateNoteRequest,
    ) -> Result<NoteResponse, AppError> {
        let note = self.repo.update(id, user_id, &req.content).await?;
        Ok(NoteResponse::from(note))
    }

    pub async fn delete(&self, id: Uuid, user_id: Uuid) -> Result<(), AppError> {
        self.repo.delete(id, user_id).await?;
        Ok(())
    }
}
