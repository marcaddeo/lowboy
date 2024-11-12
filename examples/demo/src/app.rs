//
//         let router = Router::new()
//             // App routes.
//             .route("/events", get(controller::events))
//             .route("/post", post(controller::post::create))
//             .route("/", get(controller::home))
//             // Previous routes require authentication.
//             .route_layer(login_required!(App, login_url = "/login"))
//             // Static assets.
//             .nest_service("/static", ServeDir::new("static"))
//             // Auth routes.
//             .route("/register", get(controller::auth::register_form))
//             .route("/register", post(controller::auth::register))
//             .route("/login", get(controller::auth::form))
//             .route("/login", post(controller::auth::login))
//             .route("/login/oauth", get(controller::auth::oauth))
//             .route("/logout", get(controller::auth::logout))
//             .layer(middleware::map_response_with_state(
//                 self.clone(),
//                 view::render_view,
//             ))
//             .layer(MessagesManagerLayer)
//             .layer(auth_layer)
//             .with_state(self);
