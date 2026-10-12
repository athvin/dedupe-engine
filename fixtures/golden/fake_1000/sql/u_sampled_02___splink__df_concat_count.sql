-- templated: __splink__df_concat_count
-- physical: __splink__df_concat_count_69d9f91fb
CREATE TABLE __splink__df_concat_count_69d9f91fb AS
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
