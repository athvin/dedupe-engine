-- templated: __splink__clustering_output_final
-- physical: __splink__clustering_output_final_04f97b99e
CREATE TABLE __splink__clustering_output_final_04f97b99e AS
select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_1_a82c6ab21

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_2_5d5b5d36b

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_stable_3_120110ec6

UNION ALL

select node_id as node_id, representative as cluster_id
            from __splink__representatives_unstable_3_916f200dd
