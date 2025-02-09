-- Add up migration script here
CREATE TABLE IF NOT EXISTS storages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    name VARCHAR(255) NOT NULL UNIQUE COLLATE ignoreCase,
    storage_type TEXT NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    config JSONB NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
);



CREATE TABLE IF NOT EXISTS repositories (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid (),
    storage_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL COLLATE ignoreCase,
    repository_type VARCHAR(255) NOT NULL,
    visibility VARCHAR(255) NOT NULL DEFAULT 'Public',
    active BOOLEAN NOT NULL DEFAULT TRUE,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT fk_repositories_storage_id FOREIGN KEY (storage_id) REFERENCES storages (id) ON DELETE CASCADE,
    CONSTRAINT unique_repository_name UNIQUE (storage_id, name)
);

CREATE TABLE IF NOT EXISTS repository_configs (
    id SERIAL PRIMARY KEY,
    repository_id UUID NOT NULL,
    key VARCHAR(255) NOT NULL,
    value JSONB NOT NULL,
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT CURRENT_TIMESTAMP,

    CONSTRAINT fk_repository_configs_repository_id FOREIGN KEY (repository_id) REFERENCES repositories (id) ON DELETE CASCADE,
    CONSTRAINT unique_repository_config_key UNIQUE (repository_id, key)
);

CREATE TABLE IF NOT EXISTS hostnames (
    id SERIAL PRIMARY KEY,
    repository_id         UUID
        constraint fk_repositories_hostnames
            references repositories
            on delete cascade,
    storage_id         UUID
        constraint fk_storages_hostnames
            references storages
            on delete cascade,
    -- Ensure either repository_id or storage_id is set, but not both
    CONSTRAINT one_of_repository_or_storage CHECK (
        (repository_id IS NOT NULL AND storage_id IS NULL) OR
        (repository_id IS NULL AND storage_id IS NOT NULL)
    ),
    hostname           TEXT UNIQUE COLLATE ignoreCase                     NOT NULL,
    updated_at         TIMESTAMP WITH TIME ZONE default CURRENT_TIMESTAMP not null,
    created_at         TIMESTAMP WITH TIME ZONE default CURRENT_TIMESTAMP not null
);
