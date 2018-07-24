begin;

create table effects (
  id INTEGER PRIMARY KEY,
  json TEXT NOT NULL
);

create table numerical_ids (
  scope TEXT NOT NULL,
  id INTEGER NOT NULL,
  uuid TEXT NOT NULL
);

commit;
