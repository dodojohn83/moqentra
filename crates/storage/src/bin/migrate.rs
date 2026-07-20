//! Moqentra migration CLI.

use anyhow::Result;
use clap::{Parser, Subcommand};
use sqlx::PgPool;

#[derive(Parser, Debug)]
#[command(name = "moqentra-migrate", about = "Moqentra PostgreSQL migrations")]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// PostgreSQL connection URL.
    #[arg(short, long, env = "DATABASE_URL")]
    database_url: String,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Run pending migrations.
    Migrate,
    /// Validate that the database is at the latest migration.
    Validate,
    /// Print migration status.
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let pool = PgPool::connect(&cli.database_url).await?;
    let migrator = sqlx::migrate!("./migrations");

    match cli.command {
        Command::Migrate => {
            migrator.run(&pool).await?;
            println!("Migrations applied successfully.");
        }
        Command::Validate => {
            let migrations = migrator.migrations.iter().collect::<Vec<_>>();
            let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
                .fetch_one(&pool)
                .await?;
            if (applied.max(0) as usize) == migrations.len() {
                println!("Database is up to date.");
            } else {
                anyhow::bail!("Database is not at the latest migration");
            }
        }
        Command::Status => {
            let applied: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM _sqlx_migrations")
                .fetch_one(&pool)
                .await?;
            let total = migrator.migrations.len();
            println!("Applied: {}/{} migrations", applied.max(0), total);
        }
    }

    Ok(())
}
