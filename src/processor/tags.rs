use sqlx::SqlitePool;

pub struct Tags {
    conn: SqlitePool,
}

impl Tags {
    #[allow(clippy::new_ret_no_self)]
    pub fn new(conn: SqlitePool) -> Box<dyn super::Processor + Send + Sync> {
        Box::new(Tags { conn })
    }
}

#[async_trait::async_trait]
impl super::Processor for Tags {
    async fn process(&self, data: &super::ParsedFile) -> anyhow::Result<()> {
        todo!("implement me")
    }
}
