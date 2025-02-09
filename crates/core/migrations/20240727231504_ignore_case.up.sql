-- Add up migration script here
CREATE COLLATION IF NOT EXISTS ignoreCase (
  provider = 'icu',
  locale = 'und-u-ks-level2',
  deterministic = false
);