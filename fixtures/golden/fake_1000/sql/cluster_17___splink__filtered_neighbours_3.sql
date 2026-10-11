-- templated: __splink__filtered_neighbours_3
-- physical: __splink__filtered_neighbours_3_c78c4d25b
CREATE TABLE __splink__filtered_neighbours_3_c78c4d25b AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__filtered_neighbours_2_59d5641c6 as n
left join __splink__representatives_3_73ad513d8 as l
on l.node_id = n.node_id
left join __splink__representatives_3_73ad513d8 as r
on n.neighbour = r.node_id
where l.representative <> r.representative
