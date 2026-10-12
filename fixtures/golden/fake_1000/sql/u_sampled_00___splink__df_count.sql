-- templated: __splink__df_count
-- physical: __splink__df_count_9f36c7ef7
CREATE TABLE __splink__df_count_9f36c7ef7 AS
WITH

__splink__df_concat as (
    select
    "unique_id",
    "first_name",
    "surname",
    "dob",
    "city",
    "email",
    "cluster"
            from fake_1000
)
select count(*) as count
from __splink__df_concat
