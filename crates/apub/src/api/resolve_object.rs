use crate::fetcher::search::{search_query_to_object_id, SearchableObjects};
use activitypub_federation::config::Data;
use actix_web::web::{Json, Query};
use diesel::NotFound;
use lemmy_api_common::{
  context::LemmyContext,
  site::{ResolveObject, ResolveObjectResponse},
  utils::{check_private_instance, local_user_view_from_jwt},
};
use lemmy_db_schema::{newtypes::PersonId, source::local_site::LocalSite, utils::DbPool};
use lemmy_db_views::structs::{CommentView, PostView};
use lemmy_db_views_actor::structs::{CommunityView, PersonView};
use lemmy_utils::error::{LemmyError, LemmyErrorExt2, LemmyErrorType};

#[tracing::instrument(skip(context))]
pub async fn resolve_object(
  data: Query<ResolveObject>,
  context: Data<LemmyContext>,
) -> Result<Json<ResolveObjectResponse>, LemmyError> {
  let local_user_view = local_user_view_from_jwt(&data.auth, &context).await?;
  let local_site = LocalSite::read(context.pool()).await?;
  let person_id = local_user_view.person.id;
  check_private_instance(&Some(local_user_view), &local_site)?;

  let res = search_query_to_object_id(&data.q, &context)
    .await
    .with_lemmy_type(LemmyErrorType::CouldntFindObject)?;
  convert_response(res, person_id, context.pool())
    .await
    .with_lemmy_type(LemmyErrorType::CouldntFindObject)
}

async fn convert_response(
  object: SearchableObjects,
  user_id: PersonId,
  pool: &DbPool,
) -> Result<Json<ResolveObjectResponse>, LemmyError> {
  use SearchableObjects::*;
  let removed_or_deleted;
  let mut res = ResolveObjectResponse::default();
  match object {
    Person(p) => {
      removed_or_deleted = p.deleted;
      res.person = Some(PersonView::read(pool, p.id).await?)
    }
    Community(c) => {
      removed_or_deleted = c.deleted || c.removed;
      res.community = Some(CommunityView::read(pool, c.id, Some(user_id), None).await?)
    }
    Post(p) => {
      removed_or_deleted = p.deleted || p.removed;
      res.post = Some(PostView::read(pool, p.id, Some(user_id), None).await?)
    }
    Comment(c) => {
      removed_or_deleted = c.deleted || c.removed;
      res.comment = Some(CommentView::read(pool, c.id, Some(user_id)).await?)
    }
  };
  // if the object was deleted from database, dont return it
  if removed_or_deleted {
    return Err(NotFound {}.into());
  }
  Ok(Json(res))
}
