ALTER TABLE execution_observations DROP CONSTRAINT execution_observations_object_kind_check;
ALTER TABLE execution_observations ADD CONSTRAINT execution_observations_object_kind_check
    CHECK (object_kind IN ('item','job','device','logical_work','attempt','transport'));
