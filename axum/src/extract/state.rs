use async_trait::async_trait;
use axum_core::extract::{FromRef, FromRequestParts};
use http::request::Parts;
use std::{
    convert::Infallible,
    ops::{Deref, DerefMut},
};

/// Extractor for state.
///
/// See ["Accessing state in middleware"][state-from-middleware] for how to  
/// access state in middleware.
///
/// [state-from-middleware]: crate::middleware#accessing-state-in-middleware
///
/// # With `Router`
///
/// ```
/// use axum::{Router, routing::get, extract::State};
///
/// // the application state
/// //
/// // here you can put configuration, database connection pools, or whatever
/// // state you need
/// //
/// // see "When states need to implement `Clone`" for more details on why we need
/// // `#[derive(Clone)]` here.
/// #[derive(Clone)]
/// struct AppState {}
///
/// let state = AppState {};
///
/// // create a `Router` that holds our state
/// let app = Router::new()
///     .route("/", get(handler))
///     // provide the state so the router can access it
///     .with_state(state);
///
/// async fn handler(
///     // access the state via the `State` extractor
///     // extracting a state of the wrong type results in a compile error
///     State(state): State<AppState>,
/// ) {
///     // use `state`...
/// }
/// # let _: axum::routing::RouterService = app;
/// ```
///
/// # With `MethodRouter`
///
/// ```
/// use axum::{routing::get, extract::State};
///
/// #[derive(Clone)]
/// struct AppState {}
///
/// let state = AppState {};
///
/// let method_router_with_state = get(handler)
///     // provide the state so the handler can access it
///     .with_state(state);
///
/// async fn handler(State(state): State<AppState>) {
///     // use `state`...
/// }
/// # async {
/// # axum::Server::bind(&"".parse().unwrap()).serve(method_router_with_state.into_make_service()).await.unwrap();
/// # };
/// ```
///
/// # With `Handler`
///
/// ```
/// use axum::{routing::get, handler::Handler, extract::State};
///
/// #[derive(Clone)]
/// struct AppState {}
///
/// let state = AppState {};
///
/// async fn handler(State(state): State<AppState>) {
///     // use `state`...
/// }
///
/// // provide the state so the handler can access it
/// let handler_with_state = handler.with_state(state);
///
/// # async {
/// axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
///     .serve(handler_with_state.into_make_service())
///     .await
///     .expect("server failed");
/// # };
/// ```
///
/// # Substates
///
/// [`State`] only allows a single state type but you can use [`FromRef`] to extract "substates":
///
/// ```
/// use axum::{Router, routing::get, extract::{State, FromRef}};
///
/// // the application state
/// #[derive(Clone)]
/// struct AppState {
///     // that holds some api specific state
///     api_state: ApiState,
/// }
///
/// // the api specific state
/// #[derive(Clone)]
/// struct ApiState {}
///
/// // support converting an `AppState` in an `ApiState`
/// impl FromRef<AppState> for ApiState {
///     fn from_ref(app_state: &AppState) -> ApiState {
///         app_state.api_state.clone()
///     }
/// }
///
/// let state = AppState {
///     api_state: ApiState {},
/// };
///
/// let app = Router::new()
///     .route("/", get(handler))
///     .route("/api/users", get(api_users))
///     .with_state(state);
///
/// async fn api_users(
///     // access the api specific state
///     State(api_state): State<ApiState>,
/// ) {
/// }
///
/// async fn handler(
///     // we can still access to top level state
///     State(state): State<AppState>,
/// ) {
/// }
/// # let _: axum::routing::RouterService = app;
/// ```
///
/// For convenience `FromRef` can also be derived using `#[derive(FromRef)]`.
///
/// # For library authors
///
/// If you're writing a library that has an extractor that needs state, this is the recommended way
/// to do it:
///
/// ```rust
/// use axum_core::extract::{FromRequestParts, FromRef};
/// use http::request::Parts;
/// use async_trait::async_trait;
/// use std::convert::Infallible;
///
/// // the extractor your library provides
/// struct MyLibraryExtractor;
///
/// #[async_trait]
/// impl<S> FromRequestParts<S> for MyLibraryExtractor
/// where
///     // keep `S` generic but require that it can produce a `MyLibraryState`
///     // this means users will have to implement `FromRef<UserState> for MyLibraryState`
///     MyLibraryState: FromRef<S>,
///     S: Send + Sync,
/// {
///     type Rejection = Infallible;
///
///     async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
///         // get a `MyLibraryState` from a reference to the state
///         let state = MyLibraryState::from_ref(state);
///
///         // ...
///         # todo!()
///     }
/// }
///
/// // the state your library needs
/// struct MyLibraryState {
///     // ...
/// }
/// ```
///
/// # When states need to implement `Clone`
///
/// Your top level state type must implement `Clone` to be extractable with `State`:
///
/// ```
/// use axum::extract::State;
///
/// // no substates, so to extract to `State<AppState>` we must implement `Clone` for `AppState`
/// #[derive(Clone)]
/// struct AppState {}
///
/// async fn handler(State(state): State<AppState>) {
///     // ...
/// }
/// ```
///
/// This works because of [`impl<S> FromRef<S> for S where S: Clone`][`FromRef`].
///
/// This is also true if you're extracting substates, unless you _never_ extract the top level
/// state itself:
///
/// ```
/// use axum::extract::{State, FromRef};
///
/// // we never extract `State<AppState>`, just `State<InnerState>`. So `AppState` doesn't need to
/// // implement `Clone`
/// struct AppState {
///     inner: InnerState,
/// }
///
/// #[derive(Clone)]
/// struct InnerState {}
///
/// impl FromRef<AppState> for InnerState {
///     fn from_ref(app_state: &AppState) -> InnerState {
///         app_state.inner.clone()
///     }
/// }
///
/// async fn api_users(State(inner): State<InnerState>) {
///     // ...
/// }
/// ```
///
/// In general however we recommend you implement `Clone` for all your state types to avoid
/// potential type errors.
#[derive(Debug, Default, Clone, Copy)]
pub struct State<S>(pub S);

#[async_trait]
impl<OuterState, InnerState> FromRequestParts<OuterState> for State<InnerState>
where
    InnerState: FromRef<OuterState>,
    OuterState: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        _parts: &mut Parts,
        state: &OuterState,
    ) -> Result<Self, Self::Rejection> {
        let inner_state = InnerState::from_ref(state);
        Ok(Self(inner_state))
    }
}

impl<S> Deref for State<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<S> DerefMut for State<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
