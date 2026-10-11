-- templated: __splink__representatives_1
-- physical: __splink__representatives_1_0f2e058d2
CREATE TABLE __splink__representatives_1_0f2e058d2 AS
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

        from __splink__df_neighbours_3c8e6b280

        union all

        select

            representative as old_rep,
            representative,
            1 as stable

        from __splink__df_representatives_0bfbeb6fd

    ) as rep_union
    group by old_rep
)
select

    repr.node_id,
    r.representative,
    r.stable

from __splink__rep_updates as r

left join __splink__df_representatives_0bfbeb6fd as repr
on r.old_rep = repr.representative
