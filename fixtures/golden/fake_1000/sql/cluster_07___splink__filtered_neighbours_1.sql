-- templated: __splink__filtered_neighbours_1
-- physical: __splink__filtered_neighbours_1_5394db946
CREATE TABLE __splink__filtered_neighbours_1_5394db946 AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__df_neighbours_55afdff2a as n
left join __splink__representatives_1_df2354852 as l
on l.node_id = n.node_id
left join __splink__representatives_1_df2354852 as r
on n.neighbour = r.node_id
where l.representative <> r.representative
