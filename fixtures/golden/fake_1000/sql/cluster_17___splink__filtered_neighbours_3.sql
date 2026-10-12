-- templated: __splink__filtered_neighbours_3
-- physical: __splink__filtered_neighbours_3_e6a4ebf27
CREATE TABLE __splink__filtered_neighbours_3_e6a4ebf27 AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__filtered_neighbours_2_fc333f1f9 as n
left join __splink__representatives_3_afce8095c as l
on l.node_id = n.node_id
left join __splink__representatives_3_afce8095c as r
on n.neighbour = r.node_id
where l.representative <> r.representative
