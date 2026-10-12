-- templated: __splink__clustering_output_final
-- physical: __splink__clustering_output_final_6c5caf25d
CREATE TABLE __splink__clustering_output_final_6c5caf25d AS
select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_1_4e83ace62

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_2_5c15fba5a

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_3_965bbb8b8

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_unstable_3_848ebad6c
