// Recompila quando uma migration nova entra (o sqlx::migrate! embute as migrations no binário).
fn main() {
    println!("cargo:rerun-if-changed=migrations");
}
