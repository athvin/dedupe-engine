-- templated: __splink__clustering_output_final
-- physical: __splink__clustering_output_final_4d4d041f0
CREATE TABLE __splink__clustering_output_final_4d4d041f0 AS
select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_1_0394c8f2f

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_2_c416c1b1e

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_3_89544e27c

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_unstable_3_069b58ead
