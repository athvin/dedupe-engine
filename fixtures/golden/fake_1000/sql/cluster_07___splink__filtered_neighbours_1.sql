-- templated: __splink__filtered_neighbours_1
-- physical: __splink__filtered_neighbours_1_dcab6c295
CREATE TABLE __splink__filtered_neighbours_1_dcab6c295 AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__df_neighbours_532ba0c77 as n
left join __splink__representatives_1_b8b2259f8 as l
on l.node_id = n.node_id
left join __splink__representatives_1_b8b2259f8 as r
on n.neighbour = r.node_id
where l.representative <> r.representative
