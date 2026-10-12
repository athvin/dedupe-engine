-- templated: __splink__df_clustered_with_input_data
-- physical: __splink__df_clustered_with_input_data_9478d4808
CREATE TABLE __splink__df_clustered_with_input_data_9478d4808 AS
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
from __splink__clustering_output_final_04f97b99e as cc
left join __splink__df_concat
on cc.node_id = "unique_id"
