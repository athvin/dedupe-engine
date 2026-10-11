-- templated: __splink__df_clustered_with_input_data
-- physical: __splink__df_clustered_with_input_data_db5ea429e
CREATE TABLE __splink__df_clustered_with_input_data_db5ea429e AS
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
select
    cc.cluster_id,
    "unique_id", "first_name", "surname", "dob", "city", "email", "cluster"
from __splink__clustering_output_final_4d4d041f0 as cc
left join __splink__df_concat
on cc.node_id = "unique_id"
