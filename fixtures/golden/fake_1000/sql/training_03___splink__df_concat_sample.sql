-- templated: __splink__df_concat_sample
-- physical: __splink__df_concat_sample_58d63b88f
CREATE TABLE __splink__df_concat_sample_58d63b88f AS
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
select *
from __splink__df_concat
where 1=1
