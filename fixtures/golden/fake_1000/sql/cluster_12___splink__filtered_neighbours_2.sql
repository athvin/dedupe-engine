-- templated: __splink__filtered_neighbours_2
-- physical: __splink__filtered_neighbours_2_fc333f1f9
CREATE TABLE __splink__filtered_neighbours_2_fc333f1f9 AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__filtered_neighbours_1_dcab6c295 as n
left join __splink__representatives_2_4390dc74b as l
on l.node_id = n.node_id
left join __splink__representatives_2_4390dc74b as r
on n.neighbour = r.node_id
where l.representative <> r.representative
