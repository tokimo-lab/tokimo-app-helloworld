use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // CREATE SCHEMA — SchemaManager 没有 create_schema()，用 raw SQL
        manager
            .get_connection()
            .execute_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                r#"CREATE SCHEMA IF NOT EXISTS "helloworld""#.to_owned(),
            ))
            .await?;

        manager
            .create_table(
                Table::create()
                    .table((Alias::new("helloworld"), Items::Table))
                    .col(
                        ColumnDef::new(Items::Id)
                            .uuid()
                            .not_null()
                            .default(Expr::cust("gen_random_uuid()"))
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Items::Content).text().not_null())
                    .col(
                        ColumnDef::new(Items::UserId)
                            .uuid()
                            .not_null()
                            .default(Expr::cust("'00000000-0000-0000-0000-000000000000'")),
                    )
                    .col(
                        ColumnDef::new(Items::CreatedAt)
                            .timestamp_with_time_zone()
                            .not_null()
                            .default(Expr::cust("NOW()")),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("items_created_at_idx")
                    .table((Alias::new("helloworld"), Items::Table))
                    .col((Items::CreatedAt, IndexOrder::Desc))
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("items_user_id_idx")
                    .table((Alias::new("helloworld"), Items::Table))
                    .col(Items::UserId)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table((Alias::new("helloworld"), Items::Table)).to_owned())
            .await?;

        manager
            .get_connection()
            .execute_raw(sea_orm::Statement::from_string(
                sea_orm::DatabaseBackend::Postgres,
                r#"DROP SCHEMA IF EXISTS "helloworld" CASCADE"#.to_owned(),
            ))
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Items {
    Table,
    Id,
    Content,
    UserId,
    CreatedAt,
}
