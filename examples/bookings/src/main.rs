// Example rustio-admin app used to demonstrate the view layer + schema extraction.
pub struct Booking {
    pub id: Uuid,
    pub booked_at: DateTime<Utc>,
    pub customer: String,
    pub phone: String,
    pub status: String,
    pub address: String,
    pub assigned_to: String,
    pub notes: Option<String>,
    pub internal_uuid: Uuid,
}

fn main() {
    rustio_admin::Admin::new().model::<Booking>().run();
}
