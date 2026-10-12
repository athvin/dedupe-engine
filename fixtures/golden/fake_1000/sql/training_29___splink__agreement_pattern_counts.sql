-- templated: __splink__agreement_pattern_counts
-- physical: __splink__agreement_pattern_counts_6f85a7d6a
CREATE TABLE __splink__agreement_pattern_counts_6f85a7d6a AS
select
gamma_dob,
gamma_city,
gamma_email,
count(*) as agreement_pattern_count
from __splink__df_comparison_vectors_f5144c6d3
group by gamma_dob, gamma_city, gamma_email
