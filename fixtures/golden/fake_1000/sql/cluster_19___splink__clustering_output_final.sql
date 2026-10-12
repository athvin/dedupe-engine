-- templated: __splink__clustering_output_final
-- physical: __splink__clustering_output_final_323ebd3de
CREATE TABLE __splink__clustering_output_final_323ebd3de AS
select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_1_2332b806e

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_2_d6e76a59b

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_3_0adf79ed3

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_unstable_3_3a7f6737c
