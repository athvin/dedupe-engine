-- templated: __splink__agreement_pattern_counts
-- physical: __splink__agreement_pattern_counts_015ed5756
CREATE TABLE __splink__agreement_pattern_counts_015ed5756 AS
select
gamma_dob,
gamma_city,
gamma_email,
count(*) as agreement_pattern_count
from __splink__df_comparison_vectors_98d1841d7
group by gamma_dob, gamma_city, gamma_email
