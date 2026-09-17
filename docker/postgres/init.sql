-- Runs once when the local Postgres volume is first created.
-- Integration tests (`#[sqlx::test]`) create throwaway databases on this server;
-- `miracle_test` is the connection target for DATABASE_URL in test runs.
CREATE DATABASE miracle_test OWNER miracle;
