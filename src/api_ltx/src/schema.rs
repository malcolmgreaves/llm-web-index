// @generated automatically by Diesel CLI.

diesel::table! {
    job_state (job_id) {
        job_id -> Uuid,
        url -> Text,
        status -> Text,
        kind -> Text,
    }
}

diesel::table! {
    llms_txt (job_id) {
        job_id -> Uuid,
        url -> Text,
        result -> Text,
    }
}

diesel::table! {
    names (id) {
        id -> Int4,
        name -> Varchar,
    }
}

diesel::allow_tables_to_appear_in_same_query!(job_state, llms_txt, names,);
