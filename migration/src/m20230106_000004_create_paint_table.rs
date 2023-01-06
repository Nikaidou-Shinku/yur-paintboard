use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
  async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .create_table(
        Table::create()
          .table(Paint::Table)
          .if_not_exists()
          .col(
            ColumnDef::new(Paint::Id)
              .integer()
              .not_null()
              .auto_increment()
              .primary_key()
          )
          .col(ColumnDef::new(Paint::X).integer().not_null())
          .col(ColumnDef::new(Paint::Y).integer().not_null())
          .col(ColumnDef::new(Paint::Color).string_len(7).not_null())
          .col(ColumnDef::new(Paint::Uid).integer().not_null())
          .col(ColumnDef::new(Paint::Time).timestamp().not_null())
          .to_owned()
      ).await
  }

  async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
    manager
      .drop_table(
        Table::drop()
          .table(Paint::Table).to_owned()
      ).await
  }
}

#[derive(Iden)]
enum Paint {
  Table,
  Id,
  X,
  Y,
  Color,
  Uid,
  Time,
}
