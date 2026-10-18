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

#[uniffi::export]
pub fn capture_preview_json(request: String) -> String {
    serde_json::from_str::<kalandar::CapturePreviewRequest>(&request)
        .map_err(|error| error.to_string())
        .and_then(|request| {
            serde_json::to_string(&kalandar::preview_capture_rules(request))
                .map_err(|error| error.to_string())
        })
        .unwrap_or_else(kalandar_dates::json::error)
}
