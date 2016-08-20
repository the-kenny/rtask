begin;

create table effects (
  id INTEGER PRIMARY KEY,
  json TEXT NOT NULL
);

create trigger no_delete_trigger
  before delete on effects
  begin
    select raise(rollback, "delete not allowed");
  end;

commit;
