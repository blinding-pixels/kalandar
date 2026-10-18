uniffi::setup_scaffolding!();

#[uniffi::export]
pub fn capture_schedule_json(request: String) -> String {
    kalandar_dates::json::capture_schedule_json(&request)
}

#[uniffi::export]
pub fn holiday_data_json() -> String {
    serde_json::to_string(&kalandar_dates::capture::holiday_data_info())
        .unwrap_or_else(kalandar_dates::json::error)
}
