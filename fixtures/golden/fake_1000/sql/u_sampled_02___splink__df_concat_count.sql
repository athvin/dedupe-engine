-- templated: __splink__df_concat_count
-- physical: __splink__df_concat_count_4f76ac089
CREATE TABLE __splink__df_concat_count_4f76ac089 AS
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
