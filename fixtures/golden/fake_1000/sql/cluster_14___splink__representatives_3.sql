-- templated: __splink__representatives_3
-- physical: __splink__representatives_3_3343602e0
CREATE TABLE __splink__representatives_3_3343602e0 AS
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

        from __splink__filtered_neighbours_2_3ad72a6ed

        union all

        select

            representative as old_rep,
            representative,
            1 as stable

        from __splink__representatives_unstable_2_59677dd02

    ) as rep_union
    group by old_rep
)
select

    repr.node_id,
    r.representative,
    r.stable

from __splink__rep_updates as r

left join __splink__representatives_unstable_2_59677dd02 as repr
on r.old_rep = repr.representative
