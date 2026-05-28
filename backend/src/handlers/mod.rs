pub mod scenarios;
pub mod factories;
pub mod products;
pub mod demand;

use actix_web::web;

pub fn configure(cfg: &mut web::ServiceConfig) {
    scenarios::configure(cfg);
    factories::configure(cfg);
    products::configure(cfg);
    demand::configure(cfg);
}
