-- templated: __splink__filtered_neighbours_3
-- physical: __splink__filtered_neighbours_3_2086a60f6
CREATE TABLE __splink__filtered_neighbours_3_2086a60f6 AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__filtered_neighbours_2_027dc0ffc as n
left join __splink__representatives_3_62e9c3f40 as l
on l.node_id = n.node_id
left join __splink__representatives_3_62e9c3f40 as r
on n.neighbour = r.node_id
where l.representative <> r.representative
