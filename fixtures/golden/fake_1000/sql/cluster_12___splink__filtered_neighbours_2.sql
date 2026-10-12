-- templated: __splink__filtered_neighbours_2
-- physical: __splink__filtered_neighbours_2_2669f7a2b
CREATE TABLE __splink__filtered_neighbours_2_2669f7a2b AS
select
    l.representative as node_rep,
    n.node_id,
    n.neighbour,
    r.representative as neighbour_rep
from __splink__filtered_neighbours_1_5394db946 as n
left join __splink__representatives_2_8467866eb as l
on l.node_id = n.node_id
left join __splink__representatives_2_8467866eb as r
on n.neighbour = r.node_id
where l.representative <> r.representative
