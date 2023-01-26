pub use sea_orm_migration::prelude::*;

mod m20230126_000001_create_board_table;
mod m20230126_000002_create_paint_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
  fn migrations() -> Vec<Box<dyn MigrationTrait>> {
    vec![
      Box::new(m20230126_000001_create_board_table::Migration),
      Box::new(m20230126_000002_create_paint_table::Migration),
    ]
  }
}
