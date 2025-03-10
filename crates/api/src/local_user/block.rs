use crate::Perform;
use actix_web::web::Data;
use lemmy_api_common::{
  context::LemmyContext,
  person::{BlockPerson, BlockPersonResponse},
  utils::local_user_view_from_jwt,
};
use lemmy_db_schema::{
  source::person_block::{PersonBlock, PersonBlockForm},
  traits::Blockable,
};
use lemmy_db_views_actor::structs::PersonView;
use lemmy_utils::error::{LemmyError, LemmyErrorExt, LemmyErrorType};

#[async_trait::async_trait(?Send)]
impl Perform for BlockPerson {
  type Response = BlockPersonResponse;

  #[tracing::instrument(skip(context))]
  async fn perform(&self, context: &Data<LemmyContext>) -> Result<BlockPersonResponse, LemmyError> {
    let data: &BlockPerson = self;
    let local_user_view = local_user_view_from_jwt(&data.auth, context).await?;

    let target_id = data.person_id;
    let person_id = local_user_view.person.id;

    // Don't let a person block themselves
    if target_id == person_id {
      return Err(LemmyErrorType::CantBlockYourself)?;
    }

    let person_block_form = PersonBlockForm {
      person_id,
      target_id,
    };

    let target_person_view = PersonView::read(context.pool(), target_id).await?;

    if target_person_view.person.admin {
      return Err(LemmyErrorType::CantBlockAdmin)?;
    }

    if data.block {
      PersonBlock::block(context.pool(), &person_block_form)
        .await
        .with_lemmy_type(LemmyErrorType::PersonBlockAlreadyExists)?;
    } else {
      PersonBlock::unblock(context.pool(), &person_block_form)
        .await
        .with_lemmy_type(LemmyErrorType::PersonBlockAlreadyExists)?;
    }

    Ok(BlockPersonResponse {
      person_view: target_person_view,
      blocked: data.block,
    })
  }
}
