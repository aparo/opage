use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use oas3::{spec::Operation, Spec};
use tracing::{error, info};

use crate::utils::config::Config;

use super::{
    component::object_definition::types::ObjectDatabase,
    path::{default_request, websocket_request},
};
