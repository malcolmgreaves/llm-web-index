// @generated automatically by Diesel CLI.

pub mod sql_types {
    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "job_kind"))]
    pub struct JobKind;

    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "job_status"))]
    pub struct JobStatus;

    #[derive(diesel::query_builder::QueryId, diesel::sql_types::SqlType)]
    #[diesel(postgres_type(name = "result_status"))]
    pub struct ResultStatus;
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::JobStatus;
    use super::sql_types::JobKind;

    job_state (job_id) {
        job_id -> Uuid,
        url -> Text,
        status -> JobStatus,
        kind -> JobKind,
        llms_txt -> Nullable<Text>,
        created_at -> Timestamptz,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use super::sql_types::ResultStatus;

    llms_txt (job_id) {
        job_id -> Uuid,
        url -> Text,
        result_data -> Text,
        result_status -> ResultStatus,
        created_at -> Timestamptz,
        html_compress -> Bytea,
        #[max_length = 32]
        html_checksum -> Varchar,
    }
}

diesel::allow_tables_to_appear_in_same_query!(job_state, llms_txt,);
