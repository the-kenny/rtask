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

create trigger no_upate_trigger
  before update on effects
  begin
    select raise(rollback, "update not allowed");
  end;


commit;
