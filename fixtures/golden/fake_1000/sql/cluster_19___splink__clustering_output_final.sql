-- templated: __splink__clustering_output_final
-- physical: __splink__clustering_output_final_a95f651a5
CREATE TABLE __splink__clustering_output_final_a95f651a5 AS
select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_1_47d6282d5

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_2_67a2ea33f

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_3_1015fa327

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_unstable_3_6854b2b27
