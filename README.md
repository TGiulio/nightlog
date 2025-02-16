# NightLog

NightLog is a serverless application to save and manage sky objects observation.
## Introduction
In this repo there is a collection of AWS lambda made with the Rust SAM template. The different functions manage basic CRUD operations to manage information about night sky observations. This observations are included in a Log object:

```
pub struct Observation {
    pub object_name: String,
    pub object_location: String,
    pub equipment: String,
    pub eyepiece: String,
    pub notes: String,
}

pub struct Log {
    pub _id: Option<ObjectId>,
    pub user_id: String,
    pub date: DateTime<Utc>,
    pub observation: Observation,
}
```
The whole Log is saved inside a mongoDB database collection.

## Deployment

lambdas execution environments must have the following properties defined:
```
DATABASE_URL
DATABASE_NAME
DATABASE_COLLECTION
```

The repository includes a Nix flake so that you don't have to install the Rust and AWS tools needed if you don't want to. Please refer to Nix documentation on how to use flakes and feel free to report to me if anything is missing.

`nightlog-common` is a library needed by the other functions and should **not** be doployed.

With correctly configured AWS credentials the following commands should be enough, for every lambda you want to deploy.
```
cargo lambda --release
cargo lambda deploy [--iam-role <role arn>] nightlog-<operation>
```
## Improvements
In the future I would like to add the following:
- Basic front-end
- Separate observation date from insertion one; add modification date
- Improve observation information, adding for example observer location, altitude, seeing ecc...
- Add photography information and equipment for astrophotography
- Add files management using S3 to upload photos or sketches
- Add equipment management

