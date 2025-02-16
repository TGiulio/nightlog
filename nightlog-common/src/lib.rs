use chrono::{DateTime, Utc};
use dotenv::dotenv;
use mongodb::{
    bson::{doc, oid::ObjectId, Bson},
    options::{ClientOptions, ServerApi, ServerApiVersion},
    results::{DeleteResult, UpdateResult},
    Client, Collection, Cursor,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::env;

// ENVIRONMENT

// Struct to hold configuration
pub struct Config {
    pub database_url: String,
    pub database_name: String,
    pub database_collection: String,
}

// Lazy static configuration that loads only once
pub static CONFIG: Lazy<Config> = Lazy::new(|| {
    // Load .env file
    dotenv().ok();

    Config {
        database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set in environment"),
        database_name: env::var("DATABASE_NAME").expect("DATABASE_NAME must be set in environment"),
        database_collection: env::var("DATABASE_COLLECTION")
            .expect("DATABASE_COLLECTION must be set in environment"),
    }
});

// Function to ensure environment is loaded
pub fn init() {
    // Force loading of CONFIG if it hasn't been loaded yet
    Lazy::force(&CONFIG);
}

// REQUESTS
/// Requests come into the runtime as unicode
/// strings in json format, which can map to any structure that implements `serde::Deserialize`
/// The runtime pays no attention to the contents of the request payload.

#[derive(Debug, Deserialize, Serialize)]
pub struct ObservationRequest {
    pub user_id: String,
    pub object_name: String,
    pub object_location: String,
    pub equipment: String,
    pub eyepiece: String,
    pub notes: String,
}

#[derive(Debug, Deserialize)]
pub struct GetLogRequest {
    log_id: ObjectId,
    user_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetListRequest {
    user_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteLogRequest {
    log_id: ObjectId,
    user_id: String,
}

// LOG AND COMPONENTS
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Observation {
    pub object_name: String,
    pub object_location: String,
    pub equipment: String,
    pub eyepiece: String,
    pub notes: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Log {
    pub _id: Option<ObjectId>,
    pub user_id: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub date: DateTime<Utc>,
    pub observation: Observation,
}

impl Observation {
    pub fn new(
        object_name: String,
        object_location: String,
        equipment: String,
        eyepiece: String,
        notes: String,
    ) -> Observation {
        Observation {
            object_name,
            object_location,
            equipment,
            eyepiece,
            notes,
        }
    }

    pub fn from_request(req: &ObservationRequest) -> Observation {
        Self::new(
            req.object_name.to_owned(),
            req.object_location.to_owned(),
            req.equipment.to_owned(),
            req.eyepiece.to_owned(),
            req.notes.to_owned(),
        )
    }
}

impl Log {
    pub fn new(user_id: &str, observation: &Observation) -> Log {
        Log {
            _id: Some(ObjectId::new()),
            user_id: user_id.to_owned(),
            date: Utc::now(),
            observation: observation.clone(),
        }
    }

    pub fn from_observation_request(req: &ObservationRequest) -> Log {
        let observation = Observation::from_request(req);
        Self::new(&req.user_id, &observation)
    }
}

// DATABASE FUNCTIONS
pub async fn mongodb_connection() -> Result<Client, mongodb::error::Error> {
    init();
    let mut client_options = ClientOptions::parse(&CONFIG.database_url).await?;

    // Set the server_api field of the client_options object to set the version of the Stable API on the client
    let server_api = ServerApi::builder().version(ServerApiVersion::V1).build();
    client_options.server_api = Some(server_api);
    // Get a handle to the cluster
    let client = Client::with_options(client_options)?;
    Ok(client)
}

pub async fn log_insertion(
    log: &Log,
    mongodb_client: &Client,
) -> Result<Option<ObjectId>, mongodb::error::Error> {
    let my_coll: Collection<Log> = mongodb_client
        .database(&CONFIG.database_name)
        .collection(&CONFIG.database_collection);
    let res = my_coll.insert_one(log).await?;
    let mongo_id = match res.inserted_id {
        Bson::ObjectId(oid) => Some(oid),
        _ => None,
    };
    Ok(mongo_id)
}

pub async fn log_retrieval(
    mongodb_client: &Client,
    log_req: &GetLogRequest,
) -> Result<Option<Log>, mongodb::error::Error> {
    let my_coll: Collection<Log> = mongodb_client
        .database(&CONFIG.database_name)
        .collection(&CONFIG.database_collection);
    let filter = doc! {"_id": log_req.log_id, "user_id": log_req.user_id.clone()};
    my_coll.find_one(filter).await
}

pub async fn log_replacement(
    log: &Log,
    mongodb_client: &Client,
) -> Result<UpdateResult, mongodb::error::Error> {
    let my_coll: Collection<Log> = mongodb_client
        .database(&CONFIG.database_name)
        .collection(&CONFIG.database_collection);
    let filter = doc! {"_id": log._id, "user_id": log.user_id.clone()};
    my_coll.replace_one(filter, log.to_owned()).await
}

pub async fn log_listing(
    mongodb_client: &Client,
    list_req: &GetListRequest,
) -> Result<Cursor<Log>, mongodb::error::Error> {
    let my_coll: Collection<Log> = mongodb_client
        .database(&CONFIG.database_name)
        .collection(&CONFIG.database_collection);
    let filter = doc! {"user_id": list_req.user_id.clone()};
    my_coll.find(filter).await
}

pub async fn log_deletion(
    mongodb_client: &Client,
    log_req: &DeleteLogRequest,
) -> Result<DeleteResult, mongodb::error::Error> {
    let my_coll: Collection<Log> = mongodb_client
        .database(&CONFIG.database_name)
        .collection(&CONFIG.database_collection);
    let filter = doc! {"_id": log_req.log_id, "user_id": log_req.user_id.clone()};
    my_coll.delete_one(filter).await
}

#[cfg(test)]
mod tests {
    use crate::{
        log_deletion, log_insertion, log_listing, log_replacement, log_retrieval,
        mongodb_connection, DeleteLogRequest, GetListRequest, GetLogRequest, Log,
        ObservationRequest,
    };
    use futures::TryStreamExt;

    const USER_ID: &str = "fake_id";

    #[tokio::test]
    async fn db_connection_test() {
        let client = mongodb_connection().await;
        assert!(client.is_ok());
    }

    #[test]
    fn log_creation_test() {
        let req = ObservationRequest {
            user_id: USER_ID.to_string(),
            object_name: "M31".to_string(),
            object_location: "Andromeda".to_string(),
            equipment: "Dobson 254/1250".to_string(),
            eyepiece: "25mm".to_string(),
            notes: "beautiful, even with a bad seeing".to_string(),
        };
        let log = Log::from_observation_request(&req);

        assert_eq!(req.user_id, log.user_id);
        assert_eq!(req.object_name, log.observation.object_name);
        assert_eq!(req.object_location, log.observation.object_location);
        assert_eq!(req.equipment, log.observation.equipment);
        assert_eq!(req.eyepiece, log.observation.eyepiece);
        assert_eq!(req.notes, log.observation.notes);
    }

    #[tokio::test]
    async fn log_insertion_test() {
        let client = mongodb_connection().await.unwrap();
        let req = ObservationRequest {
            user_id: USER_ID.to_string(),
            object_name: "M31".to_string(),
            object_location: "Andromeda".to_string(),
            equipment: "Dobson 254/1250".to_string(),
            eyepiece: "25mm".to_string(),
            notes: "beautiful, even with a bad seeing".to_string(),
        };
        let log = Log::from_observation_request(&req);
        let res;
        match log_insertion(&log, &client).await {
            Ok(option_id) => res = option_id,
            Err(e) => {
                println!("res is error: {:?}", e);
                panic!();
            }
        };
        let res = res.unwrap();
        let get_req = GetLogRequest {
            user_id: USER_ID.to_string(),
            log_id: res.clone(),
        };
        let saved = log_retrieval(&client, &get_req).await.unwrap();
        assert!(saved.is_some());
        let delete_req = DeleteLogRequest {
            user_id: USER_ID.to_string(),
            log_id: res,
        };
        let deleted = log_deletion(&client, &delete_req).await.unwrap();
        assert_eq!(deleted.deleted_count, 1);
    }

    #[tokio::test]
    async fn log_deletion_test() {
        let client = mongodb_connection().await.unwrap();
        let req = ObservationRequest {
            user_id: USER_ID.to_string(),
            object_name: "M31".to_string(),
            object_location: "Andromeda".to_string(),
            equipment: "Dobson 254/1250".to_string(),
            eyepiece: "25mm".to_string(),
            notes: "beautiful, even with a bad seeing".to_string(),
        };
        let log = Log::from_observation_request(&req);
        let res;
        match log_insertion(&log, &client).await {
            Ok(option_id) => res = option_id,
            Err(e) => {
                println!("res is error: {:?}", e);
                panic!();
            }
        };
        let res = res.unwrap();
        let get_req = GetLogRequest {
            user_id: USER_ID.to_string(),
            log_id: res.clone(),
        };
        let saved = log_retrieval(&client, &get_req).await.unwrap();
        assert!(saved.is_some());
        let delete_req = DeleteLogRequest {
            user_id: USER_ID.to_string(),
            log_id: res,
        };
        let deleted = log_deletion(&client, &delete_req).await.unwrap();
        assert_eq!(deleted.deleted_count, 1);

        let saved = log_retrieval(&client, &get_req).await.unwrap();
        assert!(saved.is_none());
    }

    #[tokio::test]
    async fn log_retrieving_test() {
        let client = mongodb_connection().await.unwrap();
        let object_name = "M31".to_string();
        let object_location = "Andromeda".to_string();
        let equipment = "Dobson 254/1250".to_string();
        let eyepiece = "25mm".to_string();
        let notes = "beautiful, even with a bad seeing".to_string();

        let req = ObservationRequest {
            user_id: USER_ID.to_string(),
            object_name: object_name.clone(),
            object_location: object_location.clone(),
            equipment: equipment.clone(),
            eyepiece: eyepiece.clone(),
            notes: notes.clone(),
        };
        let log = Log::from_observation_request(&req);
        let res;
        match log_insertion(&log, &client).await {
            Ok(option_id) => res = option_id,
            Err(e) => {
                println!("res is error: {:?}", e);
                panic!();
            }
        };
        let res = res.unwrap();
        let get_req = GetLogRequest {
            user_id: USER_ID.to_string(),
            log_id: res.clone(),
        };
        let saved = log_retrieval(&client, &get_req).await.unwrap().unwrap();

        assert_eq!(saved.user_id, USER_ID.to_string());
        assert_eq!(saved.observation.object_name, object_name);
        assert_eq!(saved.observation.object_location, object_location);
        assert_eq!(saved.observation.equipment, equipment);
        assert_eq!(saved.observation.eyepiece, eyepiece);
        assert_eq!(saved.observation.notes, notes);

        let delete_req = DeleteLogRequest {
            user_id: USER_ID.to_string(),
            log_id: res,
        };
        let deleted = log_deletion(&client, &delete_req).await.unwrap();
        assert_eq!(deleted.deleted_count, 1);
    }

    #[tokio::test]
    async fn log_replacing_test() {
        let client = mongodb_connection().await.unwrap();
        let req_1 = ObservationRequest {
            user_id: USER_ID.to_string(),
            object_name: "M31".to_string(),
            object_location: "Andromeda".to_string(),
            equipment: "Dobson 254/1250".to_string(),
            eyepiece: "25mm".to_string(),
            notes: "beautiful, even with a bad seeing".to_string(),
        };
        let log = Log::from_observation_request(&req_1);
        let res;
        match log_insertion(&log, &client).await {
            Ok(option_id) => res = option_id,
            Err(e) => {
                println!("res is error: {:?}", e);
                panic!();
            }
        };
        let res = res.unwrap();
        let get_req = GetLogRequest {
            user_id: USER_ID.to_string(),
            log_id: res.clone(),
        };
        let saved = log_retrieval(&client, &get_req).await.unwrap();
        assert!(saved.is_some());

        //replace
        let req_2 = ObservationRequest {
            user_id: USER_ID.to_string(),
            object_name: "M1".to_string(),
            object_location: "Taurus".to_string(),
            equipment: "Dobson 254/1200".to_string(),
            eyepiece: "10mm".to_string(),
            notes: "crab nebula".to_string(),
        };
        let mut log_2 = Log::from_observation_request(&req_2);
        log_2._id = Some(res.clone());
        let rep = log_replacement(&log_2, &client).await.unwrap();
        assert_eq!(rep.modified_count, 1);

        let replaced = log_retrieval(&client, &get_req).await.unwrap().unwrap();
        assert_eq!(replaced.user_id, log_2.user_id);
        assert_eq!(
            replaced.observation.object_name,
            log_2.observation.object_name
        );
        assert_eq!(
            replaced.observation.object_location,
            log_2.observation.object_location
        );
        assert_eq!(replaced.observation.equipment, log_2.observation.equipment);
        assert_eq!(replaced.observation.eyepiece, log_2.observation.eyepiece);
        assert_eq!(replaced.observation.notes, log_2.observation.notes);
        //delete
        let delete_req = DeleteLogRequest {
            user_id: USER_ID.to_string(),
            log_id: res,
        };
        let deleted = log_deletion(&client, &delete_req).await.unwrap();
        assert_eq!(deleted.deleted_count, 1);
    }

    #[tokio::test]
    async fn log_listing_test() {
        let client = mongodb_connection().await.unwrap();
        let req_1 = ObservationRequest {
            user_id: USER_ID.to_string(),
            object_name: "M31".to_string(),
            object_location: "Andromeda".to_string(),
            equipment: "Dobson 254/1250".to_string(),
            eyepiece: "25mm".to_string(),
            notes: "beautiful, even with a bad seeing".to_string(),
        };
        let log = Log::from_observation_request(&req_1);
        match log_insertion(&log, &client).await {
            Err(e) => {
                println!("res is error: {:?}", e);
                panic!();
            }
            _ => {}
        };

        let req_2 = ObservationRequest {
            user_id: USER_ID.to_string(),
            object_name: "M1".to_string(),
            object_location: "Taurus".to_string(),
            equipment: "Dobson 254/1200".to_string(),
            eyepiece: "10mm".to_string(),
            notes: "crab nebula".to_string(),
        };
        let log_2 = Log::from_observation_request(&req_2);
        match log_insertion(&log_2, &client).await {
            Err(e) => {
                println!("res is error: {:?}", e);
                panic!();
            }
            _ => {}
        };

        //test
        let list_req = GetListRequest {
            user_id: "fake_id".to_string(),
        };
        let cursor = log_listing(&client, &list_req).await.unwrap();
        let list = match cursor.try_collect::<Vec<Log>>().await {
            Ok(vector) => vector,
            Err(e) => {
                panic!(
                    "an error occurred in collecting user's logs in a vector: {}",
                    e
                );
            }
        };
        assert_eq!(list.len(), 2);

        //delete
        for log in list {
            let delete_req = DeleteLogRequest {
                user_id: USER_ID.to_string(),
                log_id: log._id.unwrap(),
            };
            let deleted = log_deletion(&client, &delete_req).await.unwrap();
            assert_eq!(deleted.deleted_count, 1);
        }
    }
}
