module m
contains
   subroutine s
      namelist /lppparameters/ DropStatisticsMethod, dragmodel, nTimeStepTag,   &
         DoConvertVOF2LPP,DoConvertLPP2VOF,CriteriaConvertCase, ConvertRegSizeToDiam, & 
         vol_cut,xlpp_min,xlpp_max,ylpp_min,ylpp_max,zlpp_min,zlpp_max,    &
         max_num_drop, max_num_drop_merge, maxnum_cell_drop, max_num_part, max_num_part_cross, &
         outputlpp_format,nStepConverion,AspRatioTol, &
         WallEffectSettling,output_lpp_evolution,lppbdry_cond,& 
         TwoWayCouplingFlag,LengthLPP2dp,mfLPP_ref,UnsteadyPartForce,&  
         SeedParticlesFlag, SeedWithFluidVel, & 
         AvoidSeedAtBlockBdry,AvoidSeedPartTooClose, & 
         num_part_seed, time_part_seed, & 
         xmin_part_seed, ymin_part_seed, zmin_part_seed, dmin_part_seed, & 
         xmax_part_seed, ymax_part_seed, zmax_part_seed, dmax_part_seed, &
         umin_part_seed, vmin_part_seed, wmin_part_seed, & 
         umax_part_seed, vmax_part_seed, wmax_part_seed, & 
         maxnum_diff_tag_complet, & 
         tracked_phase, ConvertMergeDrop,DoOutputLPP,vol_debris
   end subroutine
end module
