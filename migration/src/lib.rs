pub use sea_orm_migration::prelude::*;

mod m20221231_000001_create_auth_table;
mod m20221231_000002_create_session_table;
mod m20221231_000003_create_board_table;
mod m20230106_000004_create_paint_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
      Box::new(m20221231_000001_create_auth_table::Migration),
      Box::new(m20221231_000002_create_session_table::Migration),
      Box::new(m20221231_000003_create_board_table::Migration),
      Box::new(m20230106_000004_create_paint_table::Migration),
    ]
  }
}
