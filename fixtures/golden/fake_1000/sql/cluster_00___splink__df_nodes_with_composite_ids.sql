-- templated: __splink__df_nodes_with_composite_ids
-- physical: __splink__df_nodes_with_composite_ids_3b4907386
CREATE TABLE __splink__df_nodes_with_composite_ids_3b4907386 AS
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
    "unique_id" as node_id
    from __splink__df_concat
