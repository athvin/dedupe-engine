-- templated: __splink__representatives_2
-- physical: __splink__representatives_2_c6bd75d76
CREATE TABLE __splink__representatives_2_c6bd75d76 AS
WITH

__splink__rep_updates as (
    select

    old_rep,
    min(representative) as representative,
    min(stable) as stable

    from
    (

        select

            node_rep as old_rep,
            neighbour_rep as representative,
            0 as stable

        from __splink__filtered_neighbours_1_addbeeeaa

        union all

        select

            representative as old_rep,
            representative,
            1 as stable

        from __splink__representatives_unstable_1_a60781ebd

    ) as rep_union
    group by old_rep
)
select

    repr.node_id,
    r.representative,
    r.stable

from __splink__rep_updates as r

left join __splink__representatives_unstable_1_a60781ebd as repr
on r.old_rep = repr.representative
