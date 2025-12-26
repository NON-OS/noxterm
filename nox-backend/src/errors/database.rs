// BSD 3-Clause License
// Copyright (c) 2025, NØNOS - NOXTERM 

use std::fmt;
#[derive(Debug)]
pub enum DatabaseError {
    ConnectionFailed(String),
    QueryFailed(String),
    MigrationFailed(String),
    NotAvailable,
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DatabaseError::ConnectionFailed(msg) => {
                write!(f, "Database connection failed: {}", msg)
            }
            DatabaseError::QueryFailed(msg) => write!(f, "Query failed: {}", msg),
            DatabaseError::MigrationFailed(msg) => write!(f, "Migration failed: {}", msg),
            DatabaseError::NotAvailable => write!(f, "Database not available"),
        }
    }
}
