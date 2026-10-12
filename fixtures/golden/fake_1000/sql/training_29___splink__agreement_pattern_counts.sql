-- templated: __splink__agreement_pattern_counts
-- physical: __splink__agreement_pattern_counts_fcf4d7fe5
CREATE TABLE __splink__agreement_pattern_counts_fcf4d7fe5 AS
select
gamma_dob,
gamma_city,
gamma_email,
count(*) as agreement_pattern_count
from __splink__df_comparison_vectors_a96ca62d1
group by gamma_dob, gamma_city, gamma_email
