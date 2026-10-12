-- templated: __splink__agreement_pattern_counts
-- physical: __splink__agreement_pattern_counts_9d1b8bf0b
CREATE TABLE __splink__agreement_pattern_counts_9d1b8bf0b AS
select
gamma_dob,
gamma_city,
gamma_email,
count(*) as agreement_pattern_count
from __splink__df_comparison_vectors_fc95e02ed
group by gamma_dob, gamma_city, gamma_email
