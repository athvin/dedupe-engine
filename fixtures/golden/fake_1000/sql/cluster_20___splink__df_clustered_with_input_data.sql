-- templated: __splink__df_clustered_with_input_data
-- physical: __splink__df_clustered_with_input_data_5bb319263
CREATE TABLE __splink__df_clustered_with_input_data_5bb319263 AS
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
from __splink__clustering_output_final_6c5caf25d as cc
left join __splink__df_concat
on cc.node_id = "unique_id"
