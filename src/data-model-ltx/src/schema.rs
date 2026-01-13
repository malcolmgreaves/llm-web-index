// @generated automatically by Diesel CLI.

diesel::table! {
    use diesel::sql_types::*;
    use crate::models::{Job_status, Job_kind};

    job_state (job_id) {
        job_id -> Uuid,
        url -> Text,
        status -> Job_status,
        kind -> Job_kind,
        llms_txt -> Nullable<Text>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    use crate::models::Result_status;

    llms_txt (job_id) {
        job_id -> Uuid,
        url -> Text,
        result_data -> Text,
        result_status -> Result_status,
        created_at -> Timestamptz,
        html -> Text,
        html_checksum -> Varchar,
    }
}

diesel::allow_tables_to_appear_in_same_query!(job_state, llms_txt,);
