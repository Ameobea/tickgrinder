//! The macro manager supplies an interface for listing, creating, and executing spawner macros.
//! Spawner macros are the highest level of user control over the platform itself, allowing the automation
//! of tasks such as instance spawning, strategy deployment, backtesting, and other high-level control over
//! the platform and its modules.

import React from 'react';
import { connect } from 'dva';
import { Select } from 'antd';

import { MacroShape } from '../utils/commands';

const MacroManager = ({dispatch, definedMacros}) => {
  return (
    <Select>
      {/*TODO*/}
    </Select>
  );
};

MacroManager.propTypes = {
  definedMacros: React.PropTypes.arrayOf(React.PropTypes.shape(MacroShape)).isRequired,
  dispatch: React.PropTypes.func.isRequired,
};

function mapProps(state) {
  return {
    definedMacros: state.macros.definedMacros,
  };
}

export default connect(mapProps)(MacroManager);
