use rocket::routes;

#[rocket::launch]
fn thing() -> _ {
    rocket::build().mount("/", routes![])
}

