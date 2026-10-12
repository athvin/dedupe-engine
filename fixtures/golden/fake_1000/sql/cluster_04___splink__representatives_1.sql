-- templated: __splink__representatives_1
-- physical: __splink__representatives_1_df2354852
CREATE TABLE __splink__representatives_1_df2354852 AS
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

        from __splink__df_neighbours_55afdff2a

        union all

        select

            representative as old_rep,
            representative,
            1 as stable

        from __splink__df_representatives_510d48981

    ) as rep_union
    group by old_rep
)
select

    repr.node_id,
    r.representative,
    r.stable

from __splink__rep_updates as r

left join __splink__df_representatives_510d48981 as repr
on r.old_rep = repr.representative
