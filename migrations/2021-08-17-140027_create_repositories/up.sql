CREATE TABLE repositories
(
    id              BIGINT AUTO_INCREMENT PRIMARY KEY,
    name            TEXT,
    repo_type       TEXT,
    storage         BIGINT,
    settings        TEXT,
    deploy_settings TEXT,
    security        TEXT,
    created         BIGINT

)