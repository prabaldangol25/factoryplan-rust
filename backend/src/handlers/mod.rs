pub mod scenarios;
pub mod factories;
pub mod products;
pub mod demand;
pub mod runs;
pub mod import_export;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    scenarios::configure(cfg);
    factories::configure(cfg);
    products::configure(cfg);
    demand::configure(cfg);
    runs::configure(cfg);
    import_export::configure(cfg);
}
