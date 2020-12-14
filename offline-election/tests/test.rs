pub mod mock;
use crate::mock::*;
use sp_npos_elections::*;
use sp_arithmetic::*;
use substrate_test_utils::assert_eq_uvec;


#[test]
fn float_phragmen_poc_works() {
    let candidates = vec![1, 2, 3];
    let voters = vec![
        (10, vec![1, 2]),
        (20, vec![1, 3]),
        (30, vec![2, 3]),
    ];
    let stake_of = create_stake_of(&[(10, 10), (20, 20), (30, 30), (1, 0), (2, 0), (3, 0)]);
    let mut phragmen_result = elect_float(2, 2, candidates, voters, &stake_of).unwrap();
    let winners = phragmen_result.clone().winners;
    let assignments = phragmen_result.clone().assignments;

    assert_eq_uvec!(winners, vec![(2, 40), (3, 50)]);
    assert_eq_uvec!(
		assignments,
		vec![
			(10, vec![(2, 1.0)]),
			(20, vec![(3, 1.0)]),
			(30, vec![(2, 0.5), (3, 0.5)]),
		]
	);

    let mut support_map = build_support_map_float(&mut phragmen_result, &stake_of);

    assert_eq!(
        support_map.get(&2).unwrap(),
        &_Support { own: 0.0, total: 25.0, others: vec![(10u64, 10.0), (30u64, 15.0)]}
    );
    assert_eq!(
        support_map.get(&3).unwrap(),
        &_Support { own: 0.0, total: 35.0, others: vec![(20u64, 20.0), (30u64, 15.0)]}
    );

    equalize_float(phragmen_result.assignments, &mut support_map, 0.0, 2, stake_of);

    assert_eq!(
        support_map.get(&2).unwrap(),
        &_Support { own: 0.0, total: 30.0, others: vec![(10u64, 10.0), (30u64, 20.0)]}
    );
    assert_eq!(
        support_map.get(&3).unwrap(),
        &_Support { own: 0.0, total: 30.0, others: vec![(20u64, 20.0), (30u64, 10.0)]}
    );
}

#[test]
fn phragmen_poc_works() {
    let candidates = vec![1, 2, 3];
    let voters = vec![
        (10, vec![1, 2]),
        (20, vec![1, 3]),
        (30, vec![2, 3]),
    ];

    let stake_of = create_stake_of(&[(10, 10), (20, 20), (30, 30)]);
    let ElectionResult { winners, assignments } = seq_phragmen::<_, Perbill>(
        2,
        2,
        candidates,
        voters.iter().map(|(ref v, ref vs)| (v.clone(), stake_of(v), vs.clone())).collect::<Vec<_>>(),
    ).unwrap();

    assert_eq_uvec!(winners, vec![(2, 40), (3, 50)]);
    assert_eq_uvec!(
		assignments,
		vec![
			Assignment {
				who: 10u64,
				distribution: vec![(2, Perbill::from_percent(100))],
			},
			Assignment {
				who: 20,
				distribution: vec![(3, Perbill::from_percent(100))],
			},
			Assignment {
				who: 30,
				distribution: vec![
					(2, Perbill::from_percent(100/2)),
					(3, Perbill::from_percent(100/2)),
				],
			},
		]
	);

    let mut staked = assignment_ratio_to_staked(assignments, &stake_of);
    let winners = to_without_backing(winners);
    let mut support_map = build_support_map::<AccountId>(&winners, &staked).0;

    assert_eq_uvec!(
		staked,
		vec![
			StakedAssignment {
				who: 10u64,
				distribution: vec![(2, 10)],
			},
			StakedAssignment {
				who: 20,
				distribution: vec![(3, 20)],
			},
			StakedAssignment {
				who: 30,
				distribution: vec![
					(2, 15),
					(3, 15),
				],
			},
		]
	);

    assert_eq!(
        *support_map.get(&2).unwrap(),
        Support::<AccountId> { total: 25, voters: vec![(10, 10), (30, 15)] },
    );
    assert_eq!(
        *support_map.get(&3).unwrap(),
        Support::<AccountId> { total: 35, voters: vec![(20, 20), (30, 15)] },
    );

    balance_solution(
        &mut staked,
        &mut support_map,
        0,
        2,
    );

    assert_eq_uvec!(
		staked,
		vec![
			StakedAssignment {
				who: 10u64,
				distribution: vec![(2, 10)],
			},
			StakedAssignment {
				who: 20,
				distribution: vec![(3, 20)],
			},
			StakedAssignment {
				who: 30,
				distribution: vec![
					(2, 20),
					(3, 10),
				],
			},
		]
	);

    assert_eq!(
        *support_map.get(&2).unwrap(),
        Support::<AccountId> { total: 30, voters: vec![(10, 10), (30, 20)] },
    );
    assert_eq!(
        *support_map.get(&3).unwrap(),
        Support::<AccountId> { total: 30, voters: vec![(20, 20), (30, 10)] },
    );
}

#[test]
fn phragmen_poc_2_works() {
    let candidates = vec![10, 20, 30];
    let voters = vec![
        (2, vec![10, 20, 30]),
        (4, vec![10, 20, 40]),
    ];
    let stake_of = create_stake_of(&[
        (10, 1000),
        (20, 1000),
        (30, 1000),
        (40, 1000),
        (2, 500),
        (4, 500),
    ]);

    run_and_compare::<Perbill>(candidates.clone(), voters.clone(), &stake_of, 2, 2);
    run_and_compare::<Permill>(candidates.clone(), voters.clone(), &stake_of, 2, 2);
    run_and_compare::<Percent>(candidates.clone(), voters.clone(), &stake_of, 2, 2);
    run_and_compare::<PerU16>(candidates, voters, &stake_of, 2, 2);
}

#[test]
fn phragmen_poc_3_works() {
    let candidates = vec![10, 20, 30];
    let voters = vec![
        (2, vec![10, 20, 30]),
        (4, vec![10, 20, 40]),
    ];
    let stake_of = create_stake_of(&[
        (10, 1000),
        (20, 1000),
        (30, 1000),
        (2, 50),
        (4, 1000),
    ]);

    run_and_compare::<Perbill>(candidates.clone(), voters.clone(), &stake_of, 2, 2);
    run_and_compare::<Permill>(candidates.clone(), voters.clone(), &stake_of, 2, 2);
    run_and_compare::<Percent>(candidates.clone(), voters.clone(), &stake_of, 2, 2);
    run_and_compare::<PerU16>(candidates, voters, &stake_of, 2, 2);
}

#[test]
fn phragmen_accuracy_on_large_scale_only_validators() {
    // because of this particular situation we had per_u128 and now rational128. In practice, a
    // candidate can have the maximum amount of tokens, and also supported by the maximum.
    let candidates = vec![1, 2, 3, 4, 5];
    let stake_of = create_stake_of(&[
        (1, (u64::max_value() - 1).into()),
        (2, (u64::max_value() - 4).into()),
        (3, (u64::max_value() - 5).into()),
        (4, (u64::max_value() - 3).into()),
        (5, (u64::max_value() - 2).into()),
    ]);

    let ElectionResult { winners, assignments } = seq_phragmen::<_, Perbill>(
        2,
        2,
        candidates.clone(),
        auto_generate_self_voters(&candidates)
            .iter()
            .map(|(ref v, ref vs)| (v.clone(), stake_of(v), vs.clone()))
            .collect::<Vec<_>>(),
    ).unwrap();

    assert_eq_uvec!(winners, vec![(1, 18446744073709551614u128), (5, 18446744073709551613u128)]);
    assert_eq!(assignments.len(), 2);
    check_assignments_sum(assignments);
}

#[test]
fn phragmen_accuracy_on_large_scale_validators_and_nominators() {
    let candidates = vec![1, 2, 3, 4, 5];
    let mut voters = vec![
        (13, vec![1, 3, 5]),
        (14, vec![2, 4]),
    ];
    voters.extend(auto_generate_self_voters(&candidates));
    let stake_of = create_stake_of(&[
        (1,  (u64::max_value() - 1).into()),
        (2,  (u64::max_value() - 4).into()),
        (3,  (u64::max_value() - 5).into()),
        (4,  (u64::max_value() - 3).into()),
        (5,  (u64::max_value() - 2).into()),
        (13, (u64::max_value() - 10).into()),
        (14, u64::max_value().into()),
    ]);

    let ElectionResult { winners, assignments } = seq_phragmen::<_, Perbill>(
        2,
        2,
        candidates,
        voters.iter().map(|(ref v, ref vs)| (v.clone(), stake_of(v), vs.clone())).collect::<Vec<_>>(),
    ).unwrap();

    assert_eq_uvec!(winners, vec![(2, 36893488147419103226u128), (1, 36893488147419103219u128)]);
    assert_eq!(
        assignments,
        vec![
            Assignment {
                who: 13u64,
                distribution: vec![(1, Perbill::one())],
            },
            Assignment {
                who: 14,
                distribution: vec![(2, Perbill::one())],
            },
            Assignment {
                who: 1,
                distribution: vec![(1, Perbill::one())],
            },
            Assignment {
                who: 2,
                distribution: vec![(2, Perbill::one())],
            },
        ]
    );
    check_assignments_sum(assignments);
}

#[test]
fn phragmen_accuracy_on_small_scale_self_vote() {
    let candidates = vec![40, 10, 20, 30];
    let voters = auto_generate_self_voters(&candidates);
    let stake_of = create_stake_of(&[
        (40, 0),
        (10, 1),
        (20, 2),
        (30, 1),
    ]);

    let ElectionResult { winners, assignments: _ } = seq_phragmen::<_, Perbill>(
        3,
        3,
        candidates,
        voters.iter().map(|(ref v, ref vs)| (v.clone(), stake_of(v), vs.clone())).collect::<Vec<_>>(),
    ).unwrap();

    assert_eq_uvec!(winners, vec![(20, 2), (10, 1), (30, 1)]);
}

#[test]
fn phragmen_accuracy_on_small_scale_no_self_vote() {
    let candidates = vec![40, 10, 20, 30];
    let voters = vec![
        (1, vec![10]),
        (2, vec![20]),
        (3, vec![30]),
        (4, vec![40]),
    ];
    let stake_of = create_stake_of(&[
        (40, 1000), // don't care
        (10, 1000), // don't care
        (20, 1000), // don't care
        (30, 1000), // don't care
        (4, 0),
        (1, 1),
        (2, 2),
        (3, 1),
    ]);

    let ElectionResult { winners, assignments: _ } = seq_phragmen::<_, Perbill>(
        3,
        3,
        candidates,
        voters.iter().map(|(ref v, ref vs)| (v.clone(), stake_of(v), vs.clone())).collect::<Vec<_>>(),
    ).unwrap();

    assert_eq_uvec!(winners, vec![(20, 2), (10, 1), (30, 1)]);
}

#[test]
fn phragmen_large_scale_test() {
    let candidates = vec![2, 4, 6, 8, 10, 12, 14, 16 ,18, 20, 22, 24];
    let mut voters = vec![
        (50, vec![2, 4, 6, 8, 10, 12, 14, 16 ,18, 20, 22, 24]),
    ];
    voters.extend(auto_generate_self_voters(&candidates));
    let stake_of = create_stake_of(&[
        (2,  1),
        (4,  100),
        (6,  1000000),
        (8,  100000000001000),
        (10, 100000000002000),
        (12, 100000000003000),
        (14, 400000000000000),
        (16, 400000000001000),
        (18, 18000000000000000),
        (20, 20000000000000000),
        (22, 500000000000100000),
        (24, 500000000000200000),
        (50, 990000000000000000),
    ]);

    let ElectionResult { winners, assignments } = seq_phragmen::<_, Perbill>(
        2,
        2,
        candidates,
        voters.iter().map(|(ref v, ref vs)| (v.clone(), stake_of(v), vs.clone())).collect::<Vec<_>>(),
    ).unwrap();

    assert_eq_uvec!(winners, vec![(24, 1490000000000200000u128), (22, 1490000000000100000u128)]);
    check_assignments_sum(assignments);
}

#[test]
fn phragmen_large_scale_test_2() {
    let nom_budget: u64 = 1_000_000_000_000_000_000;
    let c_budget: u64 = 4_000_000;

    let candidates = vec![2, 4];
    let mut voters = vec![(50, vec![2, 4])];
    voters.extend(auto_generate_self_voters(&candidates));

    let stake_of = create_stake_of(&[
        (2,  c_budget.into()),
        (4,  c_budget.into()),
        (50, nom_budget.into()),
    ]);

    let ElectionResult { winners, assignments } = seq_phragmen::<_, Perbill>(
        2,
        2,
        candidates,
        voters.iter().map(|(ref v, ref vs)| (v.clone(), stake_of(v), vs.clone())).collect::<Vec<_>>(),
    ).unwrap();

    assert_eq_uvec!(winners, vec![(2, 1000000000004000000u128), (4, 1000000000004000000u128)]);
    assert_eq!(
        assignments,
        vec![
            Assignment {
                who: 50u64,
                distribution: vec![
                    (2, Perbill::from_parts(500000001)),
                    (4, Perbill::from_parts(499999999))
                ],
            },
            Assignment {
                who: 2,
                distribution: vec![(2, Perbill::one())],
            },
            Assignment {
                who: 4,
                distribution: vec![(4, Perbill::one())],
            },
        ],
    );
    check_assignments_sum(assignments);
}

#[test]
fn phragmen_linear_equalize() {
    let candidates = vec![11, 21, 31, 41, 51, 61, 71];
    let voters = vec![
        (2, vec![11]),
        (4, vec![11, 21]),
        (6, vec![21, 31]),
        (8, vec![31, 41]),
        (110, vec![41, 51]),
        (120, vec![51, 61]),
        (130, vec![61, 71]),
    ];
    let stake_of = create_stake_of(&[
        (11, 1000),
        (21, 1000),
        (31, 1000),
        (41, 1000),
        (51, 1000),
        (61, 1000),
        (71, 1000),

        (2, 2000),
        (4, 1000),
        (6, 1000),
        (8, 1000),
        (110, 1000),
        (120, 1000),
        (130, 1000),
    ]);

    run_and_compare::<Perbill>(candidates, voters, &stake_of, 2, 2);
}

#[test]
fn elect_has_no_entry_barrier() {
    let candidates = vec![10, 20, 30];
    let voters = vec![
        (1, vec![10]),
        (2, vec![20]),
    ];
    let stake_of = create_stake_of(&[
        (1, 10),
        (2, 10),
    ]);

    let ElectionResult { winners, assignments: _ } = seq_phragmen::<_, Perbill>(
        3,
        3,
        candidates,
        voters.iter().map(|(ref v, ref vs)| (v.clone(), stake_of(v), vs.clone())).collect::<Vec<_>>(),
    ).unwrap();

    // 30 is elected with stake 0. The caller is responsible for stripping this.
    assert_eq_uvec!(winners, vec![
		(10, 10),
		(20, 10),
		(30, 0),
	]);
}

#[test]
fn minimum_to_elect_is_respected() {
    let candidates = vec![10, 20, 30];
    let voters = vec![
        (1, vec![10]),
        (2, vec![20]),
    ];
    let stake_of = create_stake_of(&[
        (1, 10),
        (2, 10),
    ]);

    let maybe_result = seq_phragmen::<_, Perbill>(
        10,
        10,
        candidates,
        voters.iter().map(|(ref v, ref vs)| (v.clone(), stake_of(v), vs.clone())).collect::<Vec<_>>(),
    );

    assert!(maybe_result.is_none());
}

#[test]
fn self_votes_should_be_kept() {
    let candidates = vec![5, 10, 20, 30];
    let voters = vec![
        (5, vec![5]),
        (10, vec![10]),
        (20, vec![20]),
        (1, vec![10, 20])
    ];
    let stake_of = create_stake_of(&[
        (5, 5),
        (10, 10),
        (20, 20),
        (1, 8),
    ]);

    let result = seq_phragmen::<_, Perbill>(
        2,
        2,
        candidates,
        voters.iter().map(|(ref v, ref vs)| (v.clone(), stake_of(v), vs.clone())).collect::<Vec<_>>(),
    ).unwrap();

    assert_eq!(result.winners, vec![(20, 28), (10, 18)]);
    assert_eq!(
        result.assignments,
        vec![
            Assignment { who: 10, distribution: vec![(10, Perbill::from_percent(100))] },
            Assignment { who: 20, distribution: vec![(20, Perbill::from_percent(100))] },
            Assignment { who: 1, distribution: vec![
                (10, Perbill::from_percent(50)),
                (20, Perbill::from_percent(50))
            ]
            },
        ],
    );

    let mut staked_assignments = assignment_ratio_to_staked(result.assignments, &stake_of);
    let winners = to_without_backing(result.winners);

    let (mut supports, _) = build_support_map::<AccountId>(
        &winners,
        &staked_assignments,
    );

    assert_eq!(supports.get(&5u64), None);
    assert_eq!(
        supports.get(&10u64).unwrap(),
        &Support { total: 14u128, voters: vec![(10u64, 10u128), (1u64, 4u128)] },
    );
    assert_eq!(
        supports.get(&20u64).unwrap(),
        &Support { total: 24u128, voters: vec![(20u64, 20u128), (1u64, 4u128)] },
    );

    balance_solution(
        &mut staked_assignments,
        &mut supports,
        0,
        2usize,
    );

    assert_eq!(
        supports.get(&10u64).unwrap(),
        &Support { total: 18u128, voters: vec![(10u64, 10u128), (1u64, 8u128)] },
    );
    assert_eq!(
        supports.get(&20u64).unwrap(),
        &Support { total: 20u128, voters: vec![(20u64, 20u128)] },
    );
}

#[test]
fn duplicate_target_is_ignored() {
    let candidates = vec![1, 2, 3];
    let voters = vec![
        (10, 100, vec![1, 1, 2, 3]),
        (20, 100, vec![2, 3]),
        (30, 50, vec![1, 1, 2]),
    ];

    let ElectionResult { winners, assignments } = seq_phragmen::<_, Perbill>(
        2,
        2,
        candidates,
        voters,
    ).unwrap();
    let winners = to_without_backing(winners);

    assert_eq!(winners, vec![(2), (3)]);
    assert_eq!(
        assignments
            .into_iter()
            .map(|x| (x.who, x.distribution.into_iter().map(|(w, _)| w).collect::<Vec<_>>()))
            .collect::<Vec<_>>(),
        vec![
            (10, vec![2, 3]),
            (20, vec![2, 3]),
            (30, vec![2]),
        ],
    );
}

#[test]
fn duplicate_target_is_ignored_when_winner() {
    let candidates = vec![1, 2, 3];
    let voters = vec![
        (10, 100, vec![1, 1, 2, 3]),
        (20, 100, vec![1, 2]),
    ];

    let ElectionResult { winners, assignments } = seq_phragmen::<_, Perbill>(
        2,
        2,
        candidates,
        voters,
    ).unwrap();
    let winners = to_without_backing(winners);

    assert_eq!(winners, vec![1, 2]);
    assert_eq!(
        assignments
            .into_iter()
            .map(|x| (x.who, x.distribution.into_iter().map(|(w, _)| w).collect::<Vec<_>>()))
            .collect::<Vec<_>>(),
        vec![
            (10, vec![1, 2]),
            (20, vec![1, 2]),
        ],
    );
}



