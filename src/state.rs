use std::env;
use std::sync::Arc;
use std::time::Duration;

use diesel::r2d2::{ConnectionManager, Pool, PooledConnection};
use diesel::{mysql::MysqlConnection, prelude::*};
use futures::prelude::*;
use telegram_bot::{
    types::{JsonIdResponse, Message, MessageOrChannelPost, Request},
    Api, Error as TelegramError,
};
use tokio::runtime::Handle;

use crate::stats::Stats;

/// Database connection type.
pub type DbConnection = MysqlConnection;

/// Database connection manager type.
pub type DbConnectionManager = ConnectionManager<DbConnection>;

/// Database pool type.
pub type DbPool = Pool<DbConnectionManager>;

/// Database pooled connection type.
pub type DbPooled = PooledConnection<DbConnectionManager>;

/// The global application state.
#[derive(Clone)]
pub struct State {
    /// The Telegram API client beign used.
    telegram_client: Api,

    /// The inner state.
    inner: Arc<StateInner>,
}

impl State {
    /// Initialize.
    ///
    /// This initializes the global state.
    /// Internally this creates the Telegram API client and sets up a connection,
    /// connects to the bot database and more.
    ///
    /// A handle to the Tokio runtime must be given.
    pub fn init(handle: Handle) -> State {
        State {
            telegram_client: Self::create_telegram_client(),
            inner: Arc::new(StateInner::init(handle)),
        }
    }

    /// Create a Telegram API client instance, and initiate a connection.
    fn create_telegram_client() -> Api {
        // Retrieve the Telegram bot token
        let token = env::var("TELEGRAM_BOT_TOKEN").expect("env var TELEGRAM_BOT_TOKEN not set");

        // Initiate the Telegram API client, and return
        Api::new(token)
    }

    /// Get the database connection.
    pub fn db(&self) -> &DbPool {
        &self.inner.db
    }

    /// Get the database connection.
    pub fn db_connection(&self) -> DbPooled {
        self.inner
            .db
            .get()
            .expect("failed to get database connection from pool")
    }

    /// Get the Telegram API client.
    pub fn telegram_client(&self) -> &Api {
        &self.telegram_client
    }

    /// Send a request using the Telegram API client, and track the messages the bot sends.
    /// Because the stats of this message need to be tracked, it only allows to send requests that
    /// have a `Message` as response.
    /// This function uses a fixed timeout internally.
    pub async fn telegram_send<Req>(
        &self,
        request: Req,
    ) -> Result<Option<MessageOrChannelPost>, TelegramError>
    where
        Req: Request<Response = JsonIdResponse<MessageOrChannelPost>>,
    {
        // Clone the state for use in this future
        let state = self.clone();

        // Send the message through the Telegram client, track the response for stats
        let future = self
            .telegram_client()
            .send_timeout(request, Duration::from_secs(10))
            .inspect(move |msg| {
                // Unpack message, report errors
                let msg = match msg {
                    Ok(msg) => msg,
                    Err(err) => {
                        eprintln!("Telegram send error: {}", err);
                        return;
                    }
                };

                if let Some(msg) = msg {
                    let edit_date = match msg {
                        MessageOrChannelPost::Message(msg) => msg.edit_date,
                        MessageOrChannelPost::ChannelPost(post) => post.edit_date,
                    };

                    if edit_date.is_none() {
                        state
                            .stats()
                            .increase_message_or_channel_post_stats(msg, 1, 0);
                    } else {
                        state
                            .stats()
                            .increase_message_or_channel_post_stats(msg, 0, 1);
                    }
                }
            });

        future.await
    }

    // TODO: merge with telegram_send()
    /// Send a request using the Telegram API client, and track the messages the bot sends.
    /// Because the stats of this message need to be tracked, it only allows to send requests that
    /// have a `Message` as response.
    /// This function uses a fixed timeout internally.
    pub async fn telegram_send_message<Req>(
        &self,
        request: Req,
    ) -> Result<Option<Message>, TelegramError>
    where
        Req: Request<Response = JsonIdResponse<Message>>,
    {
        // Clone the state for use in this future
        let state = self.clone();

        // Send the message through the Telegram client, track the response for stats
        let future = self
            .telegram_client()
            .send_timeout(request, Duration::from_secs(10))
            .inspect(move |msg| {
                // Unpack message, report errors
                let msg = match msg {
                    Ok(msg) => msg,
                    Err(err) => {
                        eprintln!("Telegram send error: {}", err);
                        return;
                    }
                };

                if let Some(msg) = msg {
                    if msg.edit_date.is_none() {
                        state.stats().increase_message_stats(msg, 1, 0);
                    } else {
                        state.stats().increase_message_stats(msg, 0, 1);
                    }
                }
            });

        future.await
    }

    /// Send a request using the Telegram API client, and track the messages the bot sends.
    /// This function spawns the request on the background and runs it to completion.
    /// Because the stats of this message need to be tracked, it only allows to send requests that
    /// have a `Message` as response.
    /// This function uses a fixed timeout internally.
    pub fn telegram_spawn<Req>(&self, request: Req)
    where
        Req: Request<Response = JsonIdResponse<Message>> + Send + 'static,
    {
        let cloned = self.clone();
        self.inner
            .handle
            .spawn(async move { cloned.telegram_send_message(request).await });
    }

    /// Get the stats manager.
    pub fn stats(&self) -> &Stats {
        &self.inner.stats
    }
}

/// The inner state.
struct StateInner {
    /// The database connection.
    db: Pool<ConnectionManager<MysqlConnection>>,

    /// A handle to the reactor.
    handle: Handle,

    /// The stats manager.
    stats: Stats,
}

impl StateInner {
    /// Initialize.
    ///
    /// This initializes the inner state.
    /// Internally this connects to the bot database.
    pub fn init(handle: Handle) -> StateInner {
        StateInner {
            db: Self::connection_pool(),
            handle,
            stats: Stats::new(),
        }
    }

    /// Create database connection manager.
    fn connection_manager() -> DbConnectionManager {
        // Retrieve the database connection URL
        let database_url = env::var("DATABASE_URL").expect("env var DATABASE_URL not set");

        // Test connection to database
        MysqlConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Failed to connect to database on {}", database_url));

        // Build and return connection manager
        DbConnectionManager::new(database_url)
    }

    /// Create database connection pool.
    fn connection_pool() -> DbPool {
        Pool::builder()
            .build(Self::connection_manager())
            .expect("Failed to create pool.")
    }
}
