use crate::error::{Error, Result};
use futures_util::{Stream, TryStreamExt};
use kitsune_db::{
    entity::{
        accounts, posts,
        prelude::{Accounts, Posts},
    },
    r#trait::{PermissionCheck, PostPermissionCheckExt},
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use typed_builder::TypedBuilder;
use uuid::Uuid;

#[derive(Clone, TypedBuilder)]
pub struct GetPosts {
    /// ID of the account whose posts are getting fetched
    account_id: Uuid,

    /// ID of the account that is requesting the posts
    #[builder(default)]
    fetching_account_id: Option<Uuid>,

    /// Smallest ID
    ///
    /// Used for pagination
    #[builder(default)]
    min_id: Option<Uuid>,

    /// Largest ID
    ///
    /// Used for pagination
    #[builder(default)]
    max_id: Option<Uuid>,
}

#[derive(Clone, TypedBuilder)]
pub struct AccountService {
    db_conn: DatabaseConnection,
}

impl AccountService {
    /// Get a local account by its username
    pub async fn get_local_by_username(&self, username: &str) -> Result<Option<accounts::Model>> {
        Accounts::find()
            .filter(accounts::Column::Username.eq(username))
            .filter(accounts::Column::Local.eq(true))
            .one(&self.db_conn)
            .await
            .map_err(Error::from)
    }

    /// Get a stream of posts owned by the user
    ///
    /// # Panics
    ///
    /// This should never panic. If it does, please open an issue.
    pub async fn get_posts(
        &self,
        get_posts: GetPosts,
    ) -> Result<impl Stream<Item = Result<posts::Model>> + '_> {
        let permission_check = PermissionCheck::builder()
            .fetching_account_id(get_posts.fetching_account_id)
            .build()
            .unwrap();

        let mut posts_query = Posts::find()
            .filter(posts::Column::AccountId.eq(get_posts.account_id))
            .add_permission_checks(permission_check)
            .order_by_desc(posts::Column::CreatedAt);

        if let Some(min_id) = get_posts.min_id {
            posts_query = posts_query.filter(posts::Column::Id.gt(min_id));
        }

        if let Some(max_id) = get_posts.max_id {
            posts_query = posts_query.filter(posts::Column::Id.lt(max_id));
        }

        Ok(posts_query
            .stream(&self.db_conn)
            .await?
            .map_err(Error::from))
    }
}
